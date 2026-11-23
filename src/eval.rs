//! Evaluation of parsed input: variables, commands and the presentation of results.

use crate::assume::{self, Facts};
use crate::canon::{self, add, div, mul, pow, subst};
use crate::complex;
use crate::expand::expand;
use crate::expr::{Expr, Rel};
use crate::funcs::func;
use crate::matrix;
use crate::numeric;
use crate::parser;
use crate::simplify::simplify;
use std::collections::HashMap;

pub struct Session {
    pub(crate) vars: HashMap<String, Expr>,
    pub(crate) funcs: HashMap<String, (Vec<String>, Expr)>,
    ans: Option<Expr>,
    /// Remarks of the commands of the current statement.
    pub(crate) notes: Vec<String>,
    /// The defined functions whose calls are under way, the outermost first.
    calls: Vec<String>,
    /// Digits asked for on the command line.
    pub digits: Option<u32>,
    /// Print numeric values instead of exact forms.
    pub numeric: bool,
    /// Print exact forms only.
    pub exact: bool,
    /// The input has decimal numbers, or numeric values were asked for.
    pub(crate) decimal: bool,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

type Res = Result<Expr, String>;

/// Commands whose result is printed in the form they produce.
const FORM_COMMANDS: &[&str] = &[
    "expand",
    "factor",
    "together",
    "cancel",
    "apart",
    "collect",
    "N",
    "rectform",
    "complexexpand",
    "polarform",
    "trigexpand",
    "trigreduce",
    "rewrite_exp",
    "hold",
    "series",
    "numer",
    "denom",
    "charpoly",
    "pade",
    "fourierseries",
    "tobase",
    "tofraction",
    "base_conv",
    "horner",
    "rewrite",
    "combine",
];

/// Commands whose result stays exact when the input has decimal numbers.
const EXACT_COMMANDS: &[&str] = &[
    "rationalize",
    "identify",
    "cfrac",
    "convergents",
    "findrelation",
    "minpoly",
    "rat",
];

thread_local! {
    /// Where the stack of this thread begins and how much of it functions that call themselves
    /// may take; where that is not known their calls are counted.
    static CALL_STACK: std::cell::Cell<Option<(usize, usize)>> = const { std::cell::Cell::new(None) };
}

/// Tells that this thread has a stack with `room` bytes for functions that call themselves:
/// from here on such calls end when that room is used up, and not after 64 of them.
pub fn stack_for_calls(room: usize) {
    let here = 0u8;
    CALL_STACK.set(Some((std::ptr::addr_of!(here) as usize, room)));
}

/// A function has called itself (or another one that calls it) as deep as it may.
fn too_deep(depth: usize) -> bool {
    match CALL_STACK.get() {
        Some((top, room)) => {
            let here = 0u8;
            top.abs_diff(std::ptr::addr_of!(here) as usize) > room
        }
        None => depth >= 64,
    }
}

impl Session {
    pub fn new() -> Session {
        assume::clear_all();
        assume::set_domain(assume::Domain::Complex);
        crate::settings::reset();
        Session {
            vars: HashMap::new(),
            funcs: HashMap::new(),
            ans: None,
            notes: Vec::new(),
            calls: Vec::new(),
            digits: None,
            numeric: false,
            exact: false,
            decimal: false,
        }
    }

    /// Chooses the numbers of the session: complex (the default), real or quaternions.
    pub fn set_domain(&mut self, domain: assume::Domain) {
        assume::set_domain(domain);
    }

    /// A setting written as a statement: `[name]` or `[name, value]`. The lines tell what is
    /// set now.
    fn setting(&mut self, a: &[Expr]) -> Result<Vec<String>, String> {
        use crate::settings::{self, Setting};
        let kind = a
            .first()
            .and_then(Expr::sym_name)
            .and_then(settings::named)
            .ok_or("a setting is digits, roots, terms or verbose")?;
        let value = a.get(1);
        let word = value.and_then(Expr::sym_name);
        let usual = matches!(word, Some("default" | "auto"));
        let whole = |most: u64| -> Result<u64, String> {
            value
                .and_then(Expr::as_int)
                .and_then(rug::Integer::to_u64)
                .filter(|n| (1..=most).contains(n))
                .ok_or_else(|| {
                    format!(
                        "{} = N: a whole number from 1 to {most}, or default",
                        settings::name(kind)
                    )
                })
        };
        match kind {
            Setting::Digits if usual => self.digits = None,
            Setting::Digits => self.digits = Some(whole(u64::from(settings::MAX_DIGITS))? as u32),
            Setting::Roots if usual => settings::set_roots(None),
            Setting::Roots => {
                settings::set_roots(Some(whole(u64::from(settings::MAX_ROOTS))? as u32))
            }
            Setting::Terms if usual => settings::set_terms(None),
            Setting::Terms => settings::set_terms(Some(whole(settings::MAX_TERMS)?)),
            Setting::Verbose | Setting::Debug => {
                let most = if kind == Setting::Debug { 2 } else { 1 };
                let level = match (word, value.and_then(Expr::as_i64)) {
                    (None, None) | (Some("on" | "yes" | "true"), _) => most,
                    (Some("all"), _) | (_, Some(2)) => 2,
                    (Some("off" | "no" | "false" | "none" | "default"), _) | (_, Some(0)) => 0,
                    (_, Some(1)) => 1,
                    _ => {
                        return Err(
                            "verbose = 0, 1 or 2 (off, on, all): no steps, the steps, every detail"
                                .to_string(),
                        );
                    }
                };
                crate::trace::set_level(level);
            }
            Setting::List => return Ok(settings::listed(self.digits)),
        }
        Ok(vec![settings::shown(kind, self.digits)])
    }

    /// Evaluates every statement of `input` and returns the lines to print.
    pub fn run(&mut self, input: &str) -> Result<Vec<String>, String> {
        let parsed = {
            let funcs = &self.funcs;
            parser::parse_with(input, &|name| funcs.contains_key(name))?
        };
        // the unknown functions of dsolve and rsolve are written as calls: y(x), a(n + 1)
        let mut unknown: Vec<String> = Vec::new();
        for stmt in &parsed.stmts {
            stmt.walk(&mut |e| {
                if let Expr::Fn(n, a) = e
                    && &**n == ":="
                    && let Some(Expr::Fn(f, _)) = a.first()
                {
                    // a function defined and used in the same input
                    unknown.push(f.to_string());
                }
                if let Expr::Fn(n, a) = e
                    && &**n == ":="
                    && let [Expr::Sym(f), body] = &**a
                    && (crate::lists::lambda(body).is_some()
                        || body.fn_args("matlabfunction").is_some())
                {
                    // f := x -> expr defines the function f as well
                    unknown.push(f.to_string());
                }
                if let Expr::Fn(n, a) = e
                    && matches!(&**n, "dsolve" | "ndsolve")
                    && let Some(Expr::List(names)) = a.get(1)
                {
                    // the unknown functions of a system
                    unknown.extend(
                        names
                            .iter()
                            .filter_map(|x| x.sym_name().map(str::to_string)),
                    );
                }
                if let Expr::Fn(n, a) = e
                    && matches!(
                        &**n,
                        "dsolve"
                            | "rsolve"
                            | "funsolve"
                            | "ndsolve"
                            | "functionalderivative"
                            | "odetovectorfield"
                    )
                {
                    match a.get(1).and_then(Expr::sym_name) {
                        Some(name) => unknown.push(name.to_string()),
                        None if matches!(a.get(1), Some(Expr::List(_))) => {}
                        None if &**n != "rsolve" && &**n != "funsolve" => unknown.push(
                            a.first()
                                .and_then(primed_name)
                                .unwrap_or_else(|| "y".to_string()),
                        ),
                        None if &**n == "funsolve" => unknown.push("f".to_string()),
                        None => {}
                    }
                }
            });
        }
        unknown.retain(|u| !self.funcs.contains_key(u) && crate::names::function(u).is_none());
        let parsed = if unknown.is_empty() {
            parsed
        } else {
            let funcs = &self.funcs;
            parser::parse_with(input, &|name| {
                funcs.contains_key(name) || unknown.iter().any(|u| u == name)
            })?
        };
        if parsed.stmts.is_empty() {
            return Ok(Vec::new());
        }
        numeric::set_digits(self.digits.unwrap_or(32).max(parsed.max_digits));
        let decimal = self.numeric || parsed.decimal;
        self.decimal = decimal;
        let mut lines = Vec::new();
        let is_setting = |stmt: &Expr| stmt.fn_args("setting").is_some();
        // an input of settings alone answers with what was set (`settings` lists them all)
        let lists = |stmt: &Expr| {
            stmt.fn_args("setting")
                .and_then(|a| a.first())
                .and_then(Expr::sym_name)
                .and_then(crate::settings::named)
                == Some(crate::settings::Setting::List)
        };
        let listed = parsed.stmts.iter().any(lists);
        let alone = parsed.stmts.iter().all(is_setting);
        for stmt in &parsed.stmts {
            if let Some(setting) = stmt.fn_args("setting") {
                let set = self.setting(setting)?;
                numeric::set_digits(self.digits.unwrap_or(32).max(parsed.max_digits));
                if lists(stmt) || (alone && !listed) {
                    lines.extend(set);
                }
                continue;
            }
            self.notes.clear();
            crate::tetration::forget_missing();
            if crate::trace::on() {
                // with the steps on the lines are printed as they come, where they can be
                lines.retain(|line| !crate::trace::print(line));
                crate::trace::step(|| {
                    format!("the input is read as {}", crate::trace::written(stmt))
                });
            }
            let done = self.statement(stmt, decimal);
            lines.extend(crate::trace::take());
            // a drawing of tetration that fails without the program tet says that, not that
            // the function has no values there
            let drawn = matches!(stmt, Expr::Fn(n, _)
                if crate::plot::is_plot(n)
                    || crate::picture::is_save(n)
                    || crate::picture::is_drawing(n)
                    || matches!(&**n, "plot4d" | "view4d"));
            let (value, out) = match (done, crate::tetration::missing()) {
                (Err(why), Some(reason)) if drawn && !why.contains("could not run tet") => {
                    return Err(reason);
                }
                (done, _) => done?,
            };
            lines.extend(out);
            lines.extend(self.notes.drain(..).map(|n| format!("note: {n}")));
            self.ans = Some(value);
        }
        Ok(lines)
    }

    fn statement(&mut self, stmt: &Expr, decimal: bool) -> Result<(Expr, Vec<String>), String> {
        let decimal = decimal && !matches!(stmt, Expr::Fn(n, _) if EXACT_COMMANDS.contains(&&**n));
        match stmt {
            Expr::Fn(n, a) if &**n == "dsolve" && matches!(a.get(1), Some(Expr::List(_))) => {
                return self.dsolve_system(a, decimal);
            }
            Expr::Fn(n, a) if &**n == "ndsolve" => return self.ndsolve(a, decimal),
            Expr::Fn(n, a) if &**n == "dsolve" => return self.dsolve(a, decimal),
            Expr::Fn(n, a) if &**n == "functionalderivative" => {
                return self.functional_derivative(a, decimal);
            }
            Expr::Fn(n, a) if &**n == "odetovectorfield" => return self.vector_field(a, decimal),
            Expr::Fn(n, a) if &**n == "rsolve" => return self.rsolve(a, decimal),
            Expr::Fn(n, a) if &**n == "funsolve" => return self.funsolve(a, decimal),
            Expr::Fn(n, a) if crate::plot::is_plot(n) => {
                let (plain, words) = crate::picture::words_apart(a);
                let mut choices = crate::picture::Choices::default();
                for (name, value) in words {
                    let value = self.eval(value)?;
                    choices.set(name, Some(&value))?;
                }
                let args = self.eval_args(&plain)?;
                let lines = crate::picture::worded(crate::plot::draw(n, &args)?, &choices.look);
                return Ok((Expr::Bool(true), lines));
            }
            Expr::Fn(n, a) if crate::picture::is_save(n) => {
                return Ok((Expr::Bool(true), self.save(n, a)?));
            }
            Expr::Fn(n, a)
                if matches!(
                    &**n,
                    "disp"
                        | "fprintf"
                        | "writematrix"
                        | "writetable"
                        | "imwrite"
                        | "pretty"
                        | "arcgridwrite"
                        | "worldfilewrite"
                        | "matwrite"
                ) =>
            {
                let args = self.eval_args(a)?;
                return Ok((Expr::Bool(true), self.shown(n, &args)?));
            }
            Expr::Fn(n, a) if &**n == "plot4d" => {
                return Ok((Expr::Bool(true), self.plot4d(a, &[], None)?));
            }
            Expr::Fn(n, a) if &**n == "view4d" => {
                return Ok((Expr::Bool(true), self.view4d_shown(a)?));
            }
            Expr::Fn(n, a) if crate::picture::is_drawing(n) => {
                let (drawing, choices) = self.drawing(n, a, &[], false, false)?;
                let lines = crate::picture::show(n, &drawing, &choices)?;
                return Ok((Expr::Bool(true), lines));
            }
            Expr::Fn(n, a)
                if &**n == ":="
                    && matches!(&**a, [Expr::Sym(_), Expr::Fn(f, _)]
                        if matches!(&**f, "solve" | "qsolve")) =>
            {
                // a name gets the solutions
                let name = a[0].sym_name().unwrap_or_default();
                if crate::names::constant(name).is_some() || crate::names::function(name).is_some()
                {
                    return Err(format!("`{name}` is a reserved name"));
                }
                let (value, lines) = self.statement(&a[1], decimal)?;
                self.vars.insert(name.to_string(), value.clone());
                return Ok((value, lines));
            }
            Expr::Fn(n, a) if &**n == "qsolve" => {
                // the quaternion domain for this statement
                let before = assume::domain();
                assume::set_domain(assume::Domain::Quaternion);
                let out = self.statement(&Expr::Fn("solve".into(), a.clone()), decimal);
                assume::set_domain(before);
                return out;
            }
            _ => {}
        }
        let solve_call = matches!(stmt, Expr::Fn(n, _) if &**n == "solve");
        let (target, vars) = match stmt {
            Expr::Fn(n, a) if &**n == "solve" => {
                let ev = self.eval_args(a)?;
                let mut vars = Vec::new();
                for v in &ev[1..] {
                    match v {
                        Expr::Sym(s) => vars.push(s.to_string()),
                        Expr::List(items) => vars.extend(
                            items
                                .iter()
                                .filter_map(|i| i.sym_name().map(str::to_string)),
                        ),
                        _ => return Err("solve(equation, x) or solve([equations], [x, y])".into()),
                    }
                }
                (ev[0].clone(), vars)
            }
            _ => (self.eval(stmt)?, Vec::new()),
        };
        if !solve_call {
            if matches!(&target, Expr::Fn(n, _) if &**n == "solve") {
                // a solve call that came out of a definition
                return self.statement(&target, decimal);
            }
            // the solutions of an earlier equation are a list of values, not equations to solve
            if alternatives(&target) && !stmt.any(&|e| matches!(e, Expr::Rel(..))) {
                let lines = self.present(&target, decimal);
                return Ok((target, lines));
            }
        }
        if assume::quaternion_mode()
            && let Some(done) =
                self.quaternion_equations(stmt, &target, &vars, solve_call, decimal)?
        {
            return Ok(done);
        }
        if let Expr::Rel(op, l, r) = &target
            && (matches!(**l, Expr::List(_))
                || matches!(**r, Expr::List(_))
                || (crate::radix::text_of(l).is_some() && crate::radix::text_of(r).is_some()))
        {
            match crate::logic::truth(&target) {
                Some(truth) => {
                    let truth = Expr::Bool(truth);
                    return Ok((truth.clone(), vec![truth.to_string()]));
                }
                None if *op != Rel::Eq => {
                    return Ok((target.clone(), vec![target.to_string()]));
                }
                None => {}
            }
        }
        match &target {
            Expr::Rel(Rel::Eq, l, r)
                if matches!(**l, Expr::List(_)) || matches!(**r, Expr::List(_)) =>
            {
                // a list on one side (as from +-): one equation for each entry
                let difference = matrix::add(vec![
                    (**l).clone(),
                    matrix::mul(vec![Expr::minus_one(), (**r).clone()])?,
                ])?;
                let mut entries = Vec::new();
                leaves(&difference, &mut entries);
                if matches!(**l, Expr::List(_)) && matches!(**r, Expr::List(_)) {
                    // list against list: all the entries must agree at once
                    entries.retain(|f| !crate::simplify::is_zero(f));
                    return match entries.as_slice() {
                        [one] => self.solve_one(one, &vars, decimal),
                        _ => self.solve_many(&entries, &vars, decimal),
                    };
                }
                let (mut values, mut lines) = (Vec::new(), Vec::new());
                for f in &entries {
                    let (value, out) = self.solve_one(f, &vars, decimal)?;
                    values.push(value);
                    lines.extend(out);
                }
                Ok((Expr::list(values), lines))
            }
            Expr::Rel(Rel::Eq, l, r) => {
                let f = canon::sub((**l).clone(), (**r).clone());
                match crate::quat::parts(&f) {
                    // every component of a quaternion equation must vanish
                    Some(parts) => {
                        let eqs: Vec<Expr> = parts.into_iter().filter(|c| !c.is_zero()).collect();
                        if eqs.len() == 1 {
                            self.solve_one(&eqs[0], &vars, decimal)
                        } else {
                            self.solve_many(&eqs, &vars, decimal)
                        }
                    }
                    None => self.solve_one(&f, &vars, decimal),
                }
            }
            Expr::List(items)
                if !items.is_empty()
                    && items.iter().all(|i| matches!(i, Expr::Rel(Rel::Eq, ..))) =>
            {
                let eqs: Vec<Expr> = items
                    .iter()
                    .map(|i| match i {
                        Expr::Rel(_, l, r) => canon::sub((**l).clone(), (**r).clone()),
                        _ => unreachable!(),
                    })
                    .collect();
                if eqs.len() == 1 {
                    self.solve_one(&eqs[0], &vars, decimal)
                } else {
                    self.solve_many(&eqs, &vars, decimal)
                }
            }
            _ if solve_call => self.solve_one(&target, &vars, decimal),
            Expr::List(items) | Expr::Fn(_, items)
                if items.len() > 1
                    && matches!(&target, Expr::List(_) | Expr::Fn(..))
                    && (matches!(&target, Expr::List(_))
                        || matches!(&target, Expr::Fn(n, _) if &**n == "and"))
                    && items.iter().all(|i| matches!(i, Expr::Rel(..))) =>
            {
                self.conjunction(items, &target)
            }
            Expr::Rel(op, l, r) => {
                self.relation(*op, &canon::sub((**l).clone(), (**r).clone()), &target)
            }
            _ => {
                if target.has_fn("onesided") {
                    return Err("an operand is missing after the last sign (a one-sided point like 0+ belongs in limit)".into());
                }
                let keeps_form = matches!(stmt, Expr::Fn(n, _) if FORM_COMMANDS.contains(&&**n));
                let defines = matches!(stmt, Expr::Fn(n, _) if &**n == ":=");
                let value = if keeps_form {
                    target
                } else if defines {
                    simplify(&target)
                } else {
                    crate::simplify::simplify_told(&target)
                };
                // quantities that were multiplied or added come together as one
                let loose = value.has_fn("quantity")
                    && value.fn_args("quantity").is_none()
                    && !matches!(value, Expr::List(_) | Expr::Rel(..));
                let value = match loose.then(|| crate::units::settle(&value)).flatten() {
                    Some(one) => one,
                    None => value,
                };
                if let Some(failure) = external_failure(&value) {
                    if decimal || matches!(stmt, Expr::Fn(n, _) if &**n == "N") {
                        return Err(failure);
                    }
                    self.notes.push(failure);
                }
                if assume::real_mode()
                    && value.is_const()
                    && !matches!(value, Expr::List(_) | Expr::Rel(..) | Expr::Bool(_))
                    && numeric::evalf_digits(&value, numeric::digits())
                        .is_ok_and(|v| !v.imag().is_zero())
                {
                    self.notes
                        .push(format!("the value {value} is not a real number"));
                    return Ok((Expr::undefined(), vec!["undefined".to_string()]));
                }
                // a form that was asked for stands alone
                let lines = if keeps_form
                    && !decimal
                    && (value.has_fn("hold") || value.has_fn("ordered"))
                {
                    vec![value.to_string()]
                } else {
                    self.present(&value, decimal)
                };
                Ok((value, lines))
            }
        }
    }

    fn solve_one(
        &mut self,
        f: &Expr,
        vars: &[String],
        decimal: bool,
    ) -> Result<(Expr, Vec<String>), String> {
        let var = match vars.first() {
            Some(v) => v.clone(),
            None => match pick_vars(std::slice::from_ref(f), 1).pop() {
                Some(v) => v,
                None => {
                    let truth = Expr::Bool(crate::simplify::is_zero(f));
                    return Ok((truth.clone(), vec![truth.to_string()]));
                }
            },
        };
        let solved = crate::solve::solve(f, &var)?;
        Ok(self.show_roots(solved, &var, decimal))
    }

    /// The roots of an equation in one unknown as lines, without the ones that do not fit what
    /// is known about the unknown.
    fn show_roots(
        &mut self,
        mut solved: crate::solve::Solved,
        var: &str,
        decimal: bool,
    ) -> (Expr, Vec<String>) {
        let digits = numeric::digits();
        let mut lines = Vec::new();
        let mut items = Vec::new();
        let found = solved.roots.len();
        let facts = assume::get(var);
        if facts != Facts::default() {
            // what is known about the unknown: only such numbers count
            let params = solved.params.clone();
            solved.roots = std::mem::take(&mut solved.roots)
                .into_iter()
                .filter_map(|mut r| {
                    let fitted = fitting(facts, &r.value, &params)?;
                    if fitted != r.value {
                        // the one real number of a family, when the family does not leave it out
                        let member = |n: &i64| {
                            let at = params.iter().fold(r.value.clone(), |v, p| {
                                canon::subst_sym(&v, p, &Expr::int(*n))
                            });
                            crate::simplify::is_zero(&canon::sub(at, fitted.clone()))
                        };
                        if r.skip.iter().any(member) {
                            return None;
                        }
                        r.skip.clear();
                    }
                    r.value = fitted;
                    Some(r)
                })
                .collect();
        }
        if solved.all {
            lines.push(format!("every {var} is a solution"));
        } else if solved.roots.is_empty() {
            lines.push(if found > 0 {
                format!("no {} solution", kind(facts))
            } else {
                "no solution".to_string()
            });
        }
        for r in &solved.roots {
            items.push(Expr::eq(Expr::sym(var), r.value.clone()));
            let params: Vec<&String> = solved
                .params
                .iter()
                .filter(|p| r.value.has_sym(p))
                .collect();
            let tail = match params.len() {
                0 => String::new(),
                1 if r.skip.is_empty() => format!(", {} is an integer", params[0]),
                1 => format!(
                    ", {} is an integer other than {}",
                    params[0],
                    crate::solve::skipped(&r.skip)
                ),
                _ => {
                    let names: Vec<&str> = params.iter().map(|p| p.as_str()).collect();
                    format!(
                        ", {} and {} are integers",
                        names[..names.len() - 1].join(", "),
                        names[names.len() - 1]
                    )
                }
            };
            let mult = if r.mult > 1 {
                format!(" (multiplicity {})", r.mult)
            } else {
                String::new()
            };
            if r.approx {
                lines.push(format!("{var} ~ {}{mult}", r.value));
                continue;
            }
            if decimal {
                lines.push(format!(
                    "{var} = {}{mult}{tail}",
                    numeric::n(&r.value, digits)
                ));
                continue;
            }
            let shown = r.value.to_string();
            lines.push(format!("{var} = {shown}{mult}{tail}"));
            let plain = r.value.as_num().is_some_and(|n| n.is_int());
            if !self.exact && !plain && r.value.is_const() {
                match numeric::evalf_digits(&r.value, digits) {
                    Ok(v) => {
                        let approx = numeric::fmt_complex(&v, digits);
                        if approx != shown {
                            lines.push(format!("  ~ {approx}"));
                        }
                    }
                    Err(numeric::NumErr::External(failure)) => {
                        lines.push(format!("note: {failure}"))
                    }
                    Err(_) => {}
                }
            }
        }
        lines.extend(solved.notes.iter().map(|n| format!("note: {n}")));
        crate::solve::release(&solved.params);
        (Expr::list(items), lines)
    }

    /// The equations of a statement in the quaternion domain, where an unknown is a quaternion
    /// unless it was assumed real. `None` when no unknown is a quaternion.
    fn quaternion_equations(
        &mut self,
        stmt: &Expr,
        target: &Expr,
        vars: &[String],
        solve_call: bool,
        decimal: bool,
    ) -> Result<Option<(Expr, Vec<String>)>, String> {
        use crate::qsolve::{self, Unknown};
        use crate::quat;
        let count = match target {
            Expr::Rel(Rel::Eq, l, r)
                if !matches!(**l, Expr::List(_)) && !matches!(**r, Expr::List(_)) =>
            {
                1
            }
            // list against list: one equation for each pair of entries
            Expr::Rel(Rel::Eq, l, r)
                if matches!(**l, Expr::List(_)) && matches!(**r, Expr::List(_)) =>
            {
                let mut entries = Vec::new();
                leaves(l, &mut entries);
                entries.len()
            }
            // a list on one side: the equation for each entry, one after the other
            Expr::Rel(Rel::Eq, l, r) => {
                let Expr::Rel(Rel::Eq, raw_l, raw_r) = stmt else {
                    return Ok(None);
                };
                let listed = matches!(**l, Expr::List(_));
                let mut entries = Vec::new();
                leaves(if listed { l } else { r }, &mut entries);
                let (mut values, mut lines) = (Vec::new(), Vec::new());
                for entry in entries {
                    let one = if listed {
                        Expr::rel(Rel::Eq, entry, (**raw_r).clone())
                    } else {
                        Expr::rel(Rel::Eq, (**raw_l).clone(), entry)
                    };
                    let (value, out) = self.statement(&one, decimal)?;
                    values.push(value);
                    lines.extend(out);
                    lines.extend(self.notes.drain(..).map(|n| format!("note: {n}")));
                }
                return Ok(Some((Expr::list(values), lines)));
            }
            Expr::List(items)
                if !items.is_empty()
                    && items.iter().all(|i| {
                        matches!(i, Expr::Rel(Rel::Eq, l, r)
                            if !matches!(**l, Expr::List(_)) && !matches!(**r, Expr::List(_)))
                    }) =>
            {
                items.len()
            }
            Expr::List(_) | Expr::Rel(..) | Expr::Bool(_) => return Ok(None),
            _ if solve_call => 1,
            _ => return Ok(None),
        };
        let raw = match stmt {
            Expr::Fn(n, a) if &**n == "solve" => &a[0],
            other => other,
        };
        let mut taken = std::collections::BTreeSet::new();
        raw.collect_syms(&mut taken);
        taken.retain(|s| {
            !self.vars.contains_key(&**s) && !matches!(&**s, "ans" | "j" | "k" | "true" | "false")
        });
        target.collect_syms(&mut taken);
        let names: Vec<String> = if vars.is_empty() {
            let symbols: Vec<Expr> = taken.iter().map(|s| Expr::sym(s)).collect();
            pick_vars(&[Expr::list(symbols)], count)
        } else {
            vars.to_vec()
        };
        // the parts of a quaternion that is written out are real numbers
        let mut inside = std::collections::BTreeSet::new();
        let mut parts_of = |e: &Expr| {
            e.walk(&mut |x| {
                if let Expr::Fn(n, a) = x
                    && &**n == "quat"
                {
                    a.iter().for_each(|p| p.collect_syms(&mut inside));
                }
            })
        };
        parts_of(raw);
        raw.walk(&mut |x| {
            if let Some(value) = x.sym_name().and_then(|s| self.vars.get(s)) {
                parts_of(value);
            }
        });
        let real = |n: &String| assume::declared(n).real || inside.contains(n.as_str());
        if names.iter().all(real) {
            return Ok(None);
        }
        // a product of quaternions vanishes only where a factor does: each factor by itself
        if let Expr::Rel(Rel::Eq, l, r) = raw
            && count == 1
            && let Some(product) = match (l.is_zero(), r.is_zero()) {
                (false, true) => Some(l),
                (true, false) => Some(r),
                _ => None,
            }
            && let Expr::Mul(factors) = &**product
        {
            let has_unknown = |f: &Expr| names.iter().any(|n| f.has_sym(n));
            let divides =
                |f: &Expr| matches!(f, Expr::Pow(_, k) if !k.as_i64().is_some_and(|k| k > 0));
            let parts: Vec<&Expr> = factors.iter().filter(|f| has_unknown(f)).collect();
            if parts.len() >= 2 && !factors.iter().any(divides) {
                let (mut values, mut lines) = (Vec::new(), Vec::new());
                for part in parts {
                    let base = match part {
                        Expr::Pow(b, _) => (**b).clone(),
                        other => other.clone(),
                    };
                    let (value, out) =
                        self.statement(&Expr::rel(Rel::Eq, base, Expr::zero()), decimal)?;
                    values.push(value);
                    let notes: Vec<String> =
                        self.notes.drain(..).map(|n| format!("note: {n}")).collect();
                    for line in out.into_iter().chain(notes) {
                        if !lines.contains(&line) || line.starts_with("  ~") {
                            lines.push(line);
                        }
                    }
                }
                // (the remarks come last)
                let (notes, mut found): (Vec<String>, Vec<String>) =
                    lines.into_iter().partition(|l| l.starts_with("note:"));
                if found.iter().any(|l| l != "no solution") {
                    found.retain(|l| l != "no solution");
                }
                found.extend(notes);
                return Ok(Some((Expr::list(values), found)));
            }
        }
        let mut taken: std::collections::BTreeSet<String> =
            taken.iter().map(|s| s.to_string()).collect();
        taken.extend(names.iter().cloned());
        taken.extend(self.vars.keys().cloned());
        taken.extend(self.funcs.keys().cloned());
        // a name that the input does not use: the first free one of `stems`, or one with a number
        fn fresh(taken: &mut std::collections::BTreeSet<String>, stems: &[String]) -> String {
            let name = stems
                .iter()
                .find(|s| !taken.contains(*s))
                .cloned()
                .unwrap_or_else(|| {
                    (1..)
                        .map(|k| format!("{}{k}", stems[0]))
                        .find(|s| !taken.contains(s))
                        .expect("a free name")
                });
            taken.insert(name.clone());
            name
        }
        let mut unknowns: Vec<Unknown> = Vec::new();
        for name in &names {
            let parts = (!real(name)).then(|| {
                let tag = ["", "_", "__"]
                    .into_iter()
                    .find(|tag| (0..4).all(|d| !taken.contains(&format!("{name}{tag}{d}"))))
                    .unwrap_or("___");
                std::array::from_fn(|d| fresh(&mut taken, &[format!("{name}{tag}{d}")]))
            });
            unknowns.push(Unknown {
                name: name.clone(),
                parts,
            });
        }
        let sphere = fresh(
            &mut taken,
            &["u", "v", "w", "n", "p", "q"].map(String::from),
        );
        let component_names: Vec<&String> = unknowns
            .iter()
            .filter_map(|u| u.parts.as_ref())
            .flatten()
            .collect();
        for name in &component_names {
            assume::set(name, Facts::real());
        }
        let generic = |u: &Unknown| {
            u.parts
                .as_ref()
                .map(|p| Expr::call("quat", p.iter().map(|n| Expr::sym(n)).collect()))
        };
        // the statement once more, with each unknown as a quaternion of four real numbers
        let mut saved = Vec::new();
        for u in &unknowns {
            if let Some(q) = generic(u) {
                saved.push((u.name.clone(), self.vars.insert(u.name.clone(), q)));
            }
        }
        let bound = self.eval(raw);
        for (name, old) in saved {
            match old {
                Some(v) => self.vars.insert(name, v),
                None => self.vars.remove(&name),
            };
        }
        let result = (|| {
            let Ok(mut bound) = bound else {
                return Ok(None);
            };
            // an unknown that came in through a stored value
            for u in &unknowns {
                if let Some(q) = generic(u)
                    && bound.has_sym(&u.name)
                {
                    bound = canon::subst_sym(&bound, &u.name, &q);
                }
            }
            let difference = |l: &Expr, r: &Expr| {
                matrix::add(vec![
                    l.clone(),
                    matrix::mul(vec![Expr::minus_one(), r.clone()])?,
                ])
            };
            let sides: Vec<Expr> = match &bound {
                Expr::Rel(Rel::Eq, l, r) => vec![difference(l, r)?],
                Expr::List(items) => items
                    .iter()
                    .map(|i| match i {
                        Expr::Rel(Rel::Eq, l, r) => difference(l, r),
                        other => Ok(other.clone()),
                    })
                    .collect::<Result<_, String>>()?,
                other => vec![other.clone()],
            };
            let mut comps: Vec<[Expr; 4]> = Vec::new();
            for side in sides {
                // (a list: an equation for each of its entries)
                let mut entries = Vec::new();
                leaves(&side, &mut entries);
                for f in entries {
                    let parts = quat::lift(&f).map(|c| expand(&c));
                    if parts.iter().any(|c| c.any(&quat::is_quat)) {
                        let name = &names[0];
                        return Err(format!(
                            "no method for this equation: {name} is a quaternion here and a part of the equation takes no quaternion (after assume({name}, real) it is solved for a real {name})"
                        ));
                    }
                    comps.push(parts);
                }
            }
            let solved = match (comps.as_slice(), unknowns.as_slice()) {
                ([one], [unknown]) if unknown.parts.is_some() => {
                    let slice = self.slice_form(raw, &unknown.name);
                    qsolve::single(one, unknown, slice.as_ref(), &sphere)?
                }
                _ => {
                    let all: Vec<Expr> = comps.iter().flatten().cloned().collect();
                    qsolve::by_components(&all, &unknowns, &sphere)?
                }
            };
            Ok(Some(
                self.quaternion_lines(&unknowns, &solved, &sphere, decimal),
            ))
        })();
        for name in &component_names {
            assume::clear(name);
        }
        result
    }

    /// One equation as `G(x) = c`: its terms with the unknown `name`, worked out with a complex
    /// number in its place, and the others.
    fn slice_form(&mut self, raw: &Expr, name: &str) -> Option<(Expr, Expr)> {
        fn spread<'a>(e: &'a Expr, negated: bool, out: &mut Vec<(bool, &'a Expr)>) {
            match e {
                Expr::Add(items) => items.iter().for_each(|t| spread(t, negated, out)),
                Expr::Mul(f) if f.len() == 2 && f[0].is_minus_one() => spread(&f[1], !negated, out),
                other => out.push((negated, other)),
            }
        }
        let mut terms = Vec::new();
        match raw {
            Expr::Rel(Rel::Eq, l, r) => {
                spread(l, false, &mut terms);
                spread(r, true, &mut terms);
            }
            Expr::Rel(..) | Expr::List(_) => return None,
            other => spread(other, false, &mut terms),
        }
        let complex = [name.to_string()];
        let (mut with, mut without) = (Vec::new(), Vec::new());
        for (negated, term) in terms {
            let value = assume::with_complex(&complex, || self.eval(term)).ok()?;
            if matches!(value, Expr::List(_) | Expr::Rel(..)) {
                return None;
            }
            let uses = value.has_sym(name);
            if uses && value.any(&crate::quat::is_quat) {
                return None;
            }
            // G keeps its sign, c is the rest taken to the other side
            let flip = negated == uses;
            let signed = if flip {
                assume::with_complex(&complex, || mul(vec![Expr::minus_one(), value]))
            } else {
                value
            };
            if uses {
                with.push(signed);
            } else {
                without.push(signed);
            }
        }
        if with.is_empty() {
            return None;
        }
        let g = assume::with_complex(&complex, || add(with));
        Some((g, add(without)))
    }

    fn quaternion_lines(
        &mut self,
        unknowns: &[crate::qsolve::Unknown],
        solved: &crate::qsolve::Solved,
        sphere: &str,
        decimal: bool,
    ) -> (Expr, Vec<String>) {
        use crate::quat;
        let digits = numeric::digits();
        let listed = |names: &[&str]| match names {
            [] => String::new(),
            [one] => one.to_string(),
            [head @ .., last] => format!("{} and {last}", head.join(", ")),
        };
        let numeric_form = |v: &Expr| match quat::parts(v) {
            Some(parts) => quat::make(parts.map(|c| numeric::n(&c, digits))),
            None => numeric::n(v, digits),
        };
        let mut lines = Vec::new();
        let mut items = Vec::new();
        if solved.all {
            lines.push(match unknowns {
                [one] => format!("every {} is a solution", one.name),
                _ => "every value is a solution".to_string(),
            });
        } else if solved.solutions.is_empty() {
            lines.push("no solution".to_string());
        }
        let conditional = solved.solutions.iter().any(|s| !s.conditions.is_empty());
        for s in &solved.solutions {
            let conditions: Vec<String> =
                s.conditions.iter().map(crate::qsolve::condition).collect();
            let pairs: Vec<Expr> = unknowns
                .iter()
                .zip(&s.values)
                .map(|(u, v)| Expr::eq(Expr::sym(&u.name), v.clone()))
                .collect();
            items.push(match pairs.as_slice() {
                [one] => one.clone(),
                _ => Expr::list(pairs),
            });
            let relation = if s.approx { '~' } else { '=' };
            let show = |values: &[Expr]| {
                let parts: Vec<String> = unknowns
                    .iter()
                    .zip(values)
                    .map(|(u, v)| format!("{} {relation} {v}", u.name))
                    .collect();
                parts.join(", ")
            };
            let rounded: Vec<Expr> = s.values.iter().map(numeric_form).collect();
            let uses = |name: &str| s.values.iter().any(|v| v.has_sym(name));
            let params: Vec<&str> = solved
                .params
                .iter()
                .map(String::as_str)
                .filter(|p| uses(p))
                .collect();
            let mut tail = match params.len() {
                0 => String::new(),
                1 if s.skip.is_empty() => format!(", {} is an integer", params[0]),
                1 => format!(
                    ", {} is an integer other than {}",
                    params[0],
                    crate::solve::skipped(&s.skip)
                ),
                _ => format!(", {} are integers", listed(&params)),
            };
            if solved.sphere && uses(sphere) {
                tail.push_str(&format!(", {sphere} is any unit pure quaternion"));
            }
            if !conditions.is_empty() {
                tail.push_str(&format!(" with {}", conditions.join(", ")));
            }
            let mult = if s.mult > 1 {
                format!(" (multiplicity {})", s.mult)
            } else {
                String::new()
            };
            if decimal || s.approx {
                lines.push(format!("{}{mult}{tail}", show(&rounded)));
                continue;
            }
            let exact = show(&s.values);
            lines.push(format!("{exact}{mult}{tail}"));
            let constant = s
                .values
                .iter()
                .all(|v| v.free_syms().iter().all(|name| &**name == sphere) && !v.is_infinite());
            if !self.exact && constant {
                let approx = show(&rounded).replacen(" = ", " ~ ", 1);
                let same = approx.replace(" ~ ", " = ") == exact;
                if !same && unknowns.len() == 1 {
                    let value = approx.split_once(" ~ ").map_or("", |(_, v)| v);
                    lines.push(format!("  ~ {value}"));
                } else if !same {
                    lines.push(format!("  ~ {}", show(&rounded)));
                }
            }
        }
        let mut free: Vec<&str> = solved
            .free
            .iter()
            .map(String::as_str)
            .filter(|n| {
                solved
                    .solutions
                    .iter()
                    .any(|s| s.values.iter().any(|v| v.has_sym(n)))
            })
            .collect();
        free.sort_unstable();
        match (free.len(), !conditional) {
            (0, _) => {}
            (1, true) => lines.push(format!("{} is any real number", free[0])),
            (_, true) => lines.push(format!("{} are any real numbers", listed(&free))),
            (1, false) => lines.push(format!("{} is a real number", free[0])),
            (_, false) => lines.push(format!("{} are real numbers", listed(&free))),
        }
        if solved.sphere {
            lines.push(
                "note: a unit pure quaternion is a*i + b*j + c*k with a^2 + b^2 + c^2 = 1; its square is -1"
                    .to_string(),
            );
        }
        lines.extend(solved.notes.iter().map(|n| format!("note: {n}")));
        crate::solve::release(&solved.params);
        (Expr::list(items), lines)
    }

    /// `dsolve(equation or [equation, conditions], y, x)`.
    fn dsolve(&mut self, raw: &[Expr], decimal: bool) -> Result<(Expr, Vec<String>), String> {
        use crate::ode;
        let shown = |e: &Expr| {
            if decimal {
                numeric::n(e, numeric::digits())
            } else {
                e.clone()
            }
        };
        let usage = "dsolve(equation, y, x), for example dsolve(y'' + y = 0, y, x) or dsolve([y' = y, y(0) = 1], y, x)";
        let first = raw.first().ok_or(usage)?;
        let y = match raw.get(1) {
            Some(Expr::Sym(s)) => s.to_string(),
            Some(_) => return Err(usage.into()),
            None => primed_name(first).unwrap_or_else(|| "y".to_string()),
        };
        let x = match raw.get(2) {
            Some(Expr::Sym(s)) => s.to_string(),
            Some(_) => return Err(usage.into()),
            None => {
                let mut found = None;
                first.walk(&mut |e| {
                    if let Expr::Fn(n, a) = e
                        && **n == *y
                        && let [Expr::Sym(v)] = &a[..]
                    {
                        found.get_or_insert_with(|| v.to_string());
                    }
                });
                found.unwrap_or_else(|| {
                    if y == "x" {
                        "t".to_string()
                    } else {
                        "x".to_string()
                    }
                })
            }
        };
        // y, y(x) and their derivatives become symbols; y(c), y'(c) become #at(order, c)
        let convert = |e: &Expr| {
            map_raw(e, &mut |node| {
                let (mut order, mut base) = (0u32, node);
                while let Expr::Fn(n, a) = base
                    && &**n == "derivative'"
                {
                    order += 1;
                    base = &a[0];
                }
                if order == 0
                    && let Expr::Fn(n, a) = node
                    && &**n == "diff"
                    && (a.len() == 2 || a.len() == 3)
                    && a[1].is_sym(&x)
                {
                    order = a
                        .get(2)
                        .map_or(Some(1), |k| k.as_i64().and_then(|k| u32::try_from(k).ok()))?;
                    base = &a[0];
                }
                let name = |order: u32| {
                    if order == 0 {
                        Expr::sym(&y)
                    } else {
                        Expr::sym(&ode::marker(order))
                    }
                };
                match base {
                    Expr::Sym(s) if **s == *y && order > 0 => Some(name(order)),
                    Expr::Fn(n, a) if **n == *y && a.len() == 1 => Some(if a[0].is_sym(&x) {
                        name(order)
                    } else {
                        Expr::call("#at", vec![Expr::int(i64::from(order)), a[0].clone()])
                    }),
                    _ => None,
                }
            })
        };
        let items: Vec<&Expr> = match first {
            Expr::List(v) => v.iter().collect(),
            other => vec![other],
        };
        let mut equation = None;
        let mut conditions = Vec::new();
        for item in items {
            let value = self.eval(&convert(item))?;
            let (left, right) = match &value {
                Expr::Rel(Rel::Eq, l, r) => ((**l).clone(), (**r).clone()),
                other => (other.clone(), Expr::zero()),
            };
            match left.fn_args("#at") {
                Some([order, point]) if !right.has_fn("#at") => {
                    conditions.push((order.as_i64().unwrap_or(0) as u32, point.clone(), right))
                }
                _ if equation.is_some() => {
                    return Err("dsolve takes one equation (and conditions like y(0) = 1)".into());
                }
                _ => equation = Some(canon::sub(left, right)),
            }
        }
        let f = equation.ok_or(usage)?;
        if f.has_fn("#at") {
            return Err("conditions must have the form y(a) = b or y'(a) = b".into());
        }
        let mut notes = Vec::new();
        let (solutions, constants) = ode::dsolve(&f, &y, &x, &mut notes)?;
        self.notes.extend(notes);
        let (mut lines, mut items) = (Vec::new(), Vec::new());
        for s in solutions {
            match s {
                ode::Solution::Explicit(e) => {
                    let e = ode::apply_conditions(&e, &x, &constants, &conditions)?;
                    lines.push(format!("{y} = {}", shown(&e)));
                    items.push(Expr::eq(Expr::sym(&y), e));
                }
                ode::Solution::Implicit(l, r) => {
                    if !conditions.is_empty() {
                        self.notes.push(
                            "the conditions were not applied to the implicit solution".to_string(),
                        );
                    }
                    lines.push(format!("{} = {}", shown(&l), shown(&r)));
                    items.push(Expr::eq(l, r));
                }
            }
        }
        Ok((Expr::list(items), lines))
    }

    /// `rsolve(equation or [equation, conditions], a, n)`.
    fn rsolve(&mut self, raw: &[Expr], decimal: bool) -> Result<(Expr, Vec<String>), String> {
        use crate::recur;
        let usage = "rsolve(a(n+2) = a(n+1) + a(n), a) or rsolve([a(n+1) = 2a(n), a(0) = 3], a)";
        let (first, a) = match raw {
            [first, Expr::Sym(a), ..] => (first, a.to_string()),
            _ => return Err(usage.into()),
        };
        let n = match raw.get(2) {
            Some(Expr::Sym(s)) => s.to_string(),
            Some(_) => return Err(usage.into()),
            None => {
                let mut found = None;
                first.walk(&mut |e| {
                    if let Expr::Fn(f, args) = e
                        && **f == *a
                        && args.len() == 1
                        && let Some(s) = args[0].free_syms().into_iter().next()
                    {
                        found.get_or_insert_with(|| s.to_string());
                    }
                });
                found.ok_or(usage)?
            }
        };
        let items: Vec<&Expr> = match first {
            Expr::List(v) => v.iter().collect(),
            other => vec![other],
        };
        let mut shifts = Vec::new();
        let mut equation = None;
        let mut conditions = Vec::new();
        for item in items {
            let converted = self.sequence_terms(item, &a, &n, &mut shifts)?;
            let value = self.eval(&converted)?;
            let (left, right) = match &value {
                Expr::Rel(Rel::Eq, l, r) => ((**l).clone(), (**r).clone()),
                other => (other.clone(), Expr::zero()),
            };
            match left.fn_args("#at") {
                Some([index]) if !right.has_fn("#at") => conditions.push((index.clone(), right)),
                _ if equation.is_some() => {
                    return Err("rsolve takes one equation (and conditions like a(0) = 1)".into());
                }
                _ => equation = Some(canon::sub(left, right)),
            }
        }
        let f = equation.ok_or(usage)?;
        if f.has_fn("#at") {
            return Err("conditions must have the form a(0) = b".into());
        }
        let sol = recur::rsolve(&f, &n, &shifts, &conditions)?;
        let left = Expr::call(&a, vec![Expr::sym(&n)]);
        let shown = if decimal {
            numeric::n(&sol, numeric::digits())
        } else {
            sol.clone()
        };
        Ok((
            Expr::eq(left.clone(), sol),
            vec![format!("{left} = {shown}")],
        ))
    }

    /// `funsolve(equation, f)`: the functions that satisfy a functional equation.
    fn funsolve(&mut self, raw: &[Expr], decimal: bool) -> Result<(Expr, Vec<String>), String> {
        let usage = "funsolve(equation, f), for example funsolve(f(x) + 2f(1/x) = 3x, f)";
        let name = match raw.get(1) {
            Some(Expr::Sym(s)) => s.to_string(),
            Some(_) => return Err(usage.into()),
            None => "f".to_string(),
        };
        if self.funcs.contains_key(&name) {
            return Err(format!(
                "{name} is a defined function: the unknown needs a free name"
            ));
        }
        let value = self.eval(raw.first().ok_or(usage)?)?;
        let eq = match &value {
            Expr::Rel(Rel::Eq, l, r) => canon::sub((**l).clone(), (**r).clone()),
            other => other.clone(),
        };
        let found = crate::funeq::funsolve(&eq, &name)?;
        let left = Expr::call(&name, vec![Expr::sym(&found.var)]);
        let mut lines = Vec::new();
        let mut items = Vec::new();
        for s in &found.solutions {
            let shown = if decimal {
                numeric::n(s, numeric::digits())
            } else {
                s.clone()
            };
            lines.push(format!("{left} = {shown}"));
            items.push(Expr::eq(left.clone(), s.clone()));
        }
        self.notes.extend(found.notes);
        Ok((Expr::list(items), lines))
    }

    /// `a(n + k)` becomes a marker symbol, `a(number)` becomes `#at(number)`.
    fn sequence_terms(&mut self, e: &Expr, a: &str, n: &str, shifts: &mut Vec<i64>) -> Res {
        let mut failure = None;
        let mut calls: Vec<Expr> = Vec::new();
        e.walk(&mut |x| {
            if let Expr::Fn(f, args) = x
                && **f == *a
                && args.len() == 1
            {
                calls.push(args[0].clone());
            }
        });
        let mut table: Vec<(Expr, Expr)> = Vec::new();
        for arg in calls {
            let value = self.eval(&arg)?;
            let shift = simplify(&canon::sub(value.clone(), Expr::sym(n)));
            let replacement = match (shift.as_i64(), value.as_int()) {
                (Some(k), _) => {
                    shifts.push(k);
                    Expr::sym(&crate::recur::marker(k))
                }
                (None, Some(_)) => Expr::call("#at", vec![value]),
                _ => {
                    failure = Some(format!(
                        "{a}({arg}) is neither {a}({n} + k) nor a starting value"
                    ));
                    Expr::zero()
                }
            };
            table.push((arg, replacement));
        }
        if let Some(e) = failure {
            return Err(e);
        }
        Ok(map_raw(e, &mut |x| match x {
            Expr::Fn(f, args) if **f == *a && args.len() == 1 => table
                .iter()
                .find(|(arg, _)| *arg == args[0])
                .map(|(_, r)| r.clone()),
            _ => None,
        }))
    }

    /// Several relations that must hold together, in one real variable.
    fn conjunction(&mut self, items: &[Expr], shown: &Expr) -> Result<(Expr, Vec<String>), String> {
        let rels: Vec<(Expr, Rel)> = items
            .iter()
            .map(|i| match i {
                Expr::Rel(op, l, r) => (canon::sub((**l).clone(), (**r).clone()), *op),
                _ => unreachable!(),
            })
            .collect();
        let all: Vec<Expr> = rels.iter().map(|(f, _)| f.clone()).collect();
        match pick_vars(&all, 2).as_slice() {
            [] => {
                let mut truth = true;
                for (i, (f, op)) in rels.iter().enumerate() {
                    truth &= matches!(self.relation(*op, f, &items[i])?.0, Expr::Bool(true));
                }
                Ok((Expr::Bool(truth), vec![truth.to_string()]))
            }
            [var] => Ok((shown.clone(), crate::reals::inequalities(&rels, var)?)),
            _ => {
                let value = simplify(shown);
                Ok((value.clone(), vec![value.to_string()]))
            }
        }
    }

    /// An inequality: its truth for constants, its solution set for one real variable.
    fn relation(&mut self, op: Rel, f: &Expr, shown: &Expr) -> Result<(Expr, Vec<String>), String> {
        let vars = pick_vars(std::slice::from_ref(f), 2);
        match vars.as_slice() {
            [] => {
                let s = numeric::sign(&simplify(f))
                    .ok_or_else(|| format!("{shown} compares values that are not real"))?;
                let truth = Expr::Bool(match op {
                    Rel::Lt => s < 0,
                    Rel::Le => s <= 0,
                    Rel::Gt => s > 0,
                    Rel::Ge => s >= 0,
                    Rel::Ne => s != 0,
                    Rel::Eq => s == 0,
                });
                Ok((truth.clone(), vec![truth.to_string()]))
            }
            [var] if op == Rel::Ne => {
                let solved = crate::solve::solve(f, var)?;
                let mut lines: Vec<String> = Vec::new();
                if solved.all {
                    lines.push("no solution".to_string());
                } else if solved.roots.is_empty() {
                    lines.push(format!("every {var} is a solution"));
                } else {
                    lines.push(
                        solved
                            .roots
                            .iter()
                            .map(|r| format!("{var} != {}", r.value))
                            .collect::<Vec<_>>()
                            .join(" and "),
                    );
                    match solved.params.as_slice() {
                        [] => {}
                        [k] => lines.push(format!("{k} is an integer")),
                        ks => lines.push(format!("{} are integers", ks.join(", "))),
                    }
                }
                crate::solve::release(&solved.params);
                Ok((shown.clone(), lines))
            }
            [var] => Ok((
                shown.clone(),
                crate::reals::inequalities(&[(f.clone(), op)], var)?,
            )),
            _ => {
                let value = simplify(shown);
                Ok((value.clone(), vec![value.to_string()]))
            }
        }
    }

    fn solve_many(
        &mut self,
        eqs: &[Expr],
        vars: &[String],
        decimal: bool,
    ) -> Result<(Expr, Vec<String>), String> {
        let vars: Vec<String> = if vars.is_empty() {
            pick_vars(eqs, eqs.len())
        } else {
            vars.to_vec()
        };
        if vars.is_empty() {
            let truth = Expr::Bool(eqs.iter().all(crate::simplify::is_zero));
            return Ok((truth.clone(), vec![truth.to_string()]));
        }
        let digits = numeric::digits();
        let only_unknowns = |e: &Expr| {
            e.free_syms()
                .iter()
                .all(|s| vars.iter().any(|v| **s == **v))
        };
        let searchable = eqs.len() == vars.len() && eqs.iter().all(only_unknowns);
        // (no method: the numeric search may still find solutions)
        let (mut solved, failure) = match crate::system::solve_system(eqs, &vars) {
            Ok(solved) => (solved, None),
            Err(failure) if searchable => (Default::default(), Some(failure)),
            Err(failure) => return Err(failure),
        };
        if solved.solutions.is_empty() && !solved.empty && searchable {
            // nothing found and nothing proved: Newton's method from many starting points
            let real = vars.iter().all(|v| assume::is_real_sym(v));
            for point in crate::system::numeric_search(eqs, &vars, real, &mut Vec::new()) {
                let values = point.into_iter().map(|z| numeric::to_float_expr(z, digits));
                solved
                    .solutions
                    .push(vars.iter().cloned().zip(values).collect());
            }
            if !solved.solutions.is_empty() {
                solved.approx = true;
                solved.notes.push(
                    "found by a numeric search from many starting points: there may be other solutions"
                        .to_string(),
                );
            }
        }
        if solved.solutions.is_empty()
            && !solved.empty
            && let [var] = vars.as_slice()
            && eqs.len() > 1
            && let Ok(common) = crate::solve::common(eqs, var)
            && !common.roots.is_empty()
        {
            // one unknown in several equations: the roots of one that are roots of the others
            return Ok(self.show_roots(common, var, decimal));
        }
        if let Some(failure) = failure
            && solved.solutions.is_empty()
        {
            return Err(failure);
        }
        let mut lines = Vec::new();
        let mut items = Vec::new();
        let found = solved.solutions.len();
        let known: Vec<Facts> = vars.iter().map(|v| assume::get(v)).collect();
        if known.iter().any(|f| *f != Facts::default()) {
            // what is known about the unknowns: only such numbers count
            let params = solved.params.clone();
            solved.solutions.retain(|s| {
                s.iter()
                    .all(|(n, v)| fitting(assume::get(n), v, &params).is_some_and(|r| r == *v))
            });
        }
        if solved.solutions.is_empty() {
            let kinds: Vec<String> = known.iter().map(|f| kind(*f)).collect();
            lines.push(if found == 0 {
                "no solution".to_string()
            } else if kinds.iter().all(|k| *k == kinds[0]) {
                format!("no {} solution", kinds[0])
            } else {
                "no solution fits what is assumed about the unknowns".to_string()
            });
        }
        for s in &solved.solutions {
            items.push(Expr::list(
                s.iter()
                    .map(|(n, v)| Expr::eq(Expr::sym(n), v.clone()))
                    .collect(),
            ));
            let show = |f: &dyn Fn(&Expr) -> Option<String>| -> Option<String> {
                let parts: Option<Vec<String>> = s
                    .iter()
                    .map(|(n, v)| f(v).map(|t| format!("{n} = {t}")))
                    .collect();
                parts.map(|p| p.join(", "))
            };
            if decimal {
                lines.push(show(&|v| Some(numeric::n(v, digits).to_string())).unwrap());
                continue;
            }
            let exact = show(&|v| Some(v.to_string())).unwrap();
            let needs_approx = s.iter().all(|(_, v)| v.is_const())
                && s.iter().any(|(_, v)| {
                    !v.as_num().is_some_and(|n| n.is_int()) && !matches!(v, Expr::Float(_))
                });
            lines.push(exact.clone());
            if !self.exact && needs_approx {
                let approx = show(&|v| {
                    if v.is_const() {
                        numeric::evalf_digits(v, digits)
                            .ok()
                            .map(|x| numeric::fmt_complex(&x, digits))
                    } else {
                        Some(v.to_string())
                    }
                });
                if let Some(a) = approx
                    && a != exact
                {
                    lines.push(format!("  ~ {a}"));
                }
            }
        }
        for (at, p) in solved.params.iter().enumerate() {
            if !solved.params[..at].contains(p)
                && solved
                    .solutions
                    .iter()
                    .any(|s| s.iter().any(|(_, v)| v.has_sym(p)))
            {
                lines.push(format!("{p} is an integer"));
            }
        }
        lines.extend(solved.notes.iter().map(|n| format!("note: {n}")));
        crate::solve::release(&solved.params);
        Ok((Expr::list(items), lines))
    }

    fn present(&self, value: &Expr, decimal: bool) -> Vec<String> {
        let digits = numeric::digits();
        if let Some([size, unit]) = value.fn_args("quantity") {
            // a quantity: the value with its unit, and its digits when it is no plain number
            let rounded = Expr::call("quantity", vec![numeric::n(size, digits), unit.clone()]);
            if decimal {
                return vec![rounded.to_string()];
            }
            let mut lines = vec![value.to_string()];
            let plain = size.as_rat().is_some() && !lines[0].starts_with('(');
            if !self.exact && !plain && size.is_const() && rounded.to_string() != lines[0] {
                lines.push(format!("  ~ {rounded}"));
            }
            return lines;
        }
        if decimal {
            // (a number that was asked for with more digits keeps them)
            let mut most = digits;
            value.walk(&mut |part| {
                if let Expr::Float(number) = part {
                    most = most.max(number.digits);
                }
            });
            return vec![numeric::n(value, most).to_string()];
        }
        let mut lines = vec![value.to_string()];
        let plain = value.as_num().is_some_and(|n| n.is_int())
            || matches!(value, Expr::Float(_) | Expr::Bool(_));
        if !self.exact
            && !plain
            && value.is_const()
            && !value.is_infinite()
            && !matches!(value, Expr::List(_) | Expr::Rel(..))
            && let Ok(v) = numeric::evalf_digits(value, digits)
        {
            let approx = numeric::fmt_complex(&v, digits);
            // (the whole number that a kept form like 2^3*3^2*5 stands for tells nothing new)
            let whole = v.imag().is_zero() && v.real().is_integer() && value.has_fn("hold");
            if approx != lines[0] && !whole {
                lines.push(format!("  ~ {approx}"));
            }
        }
        lines
    }

    /// A command with units: a name of a unit in its arguments is the unit, whatever value a
    /// variable of that name has.
    fn units(&mut self, name: &str, raw: &[Expr]) -> Res {
        use crate::units;
        let mut hidden = Vec::new();
        for arg in raw {
            arg.walk(&mut |e| {
                if let Some(s) = e.sym_name()
                    && units::is_unit(s)
                    && let Some(value) = self.vars.remove(s)
                {
                    hidden.push((s.to_string(), value));
                }
            });
        }
        let raw: Vec<Expr> = raw
            .iter()
            .map(|arg| map_raw(arg, &mut |e| units::written(e)))
            .collect();
        let args = self.eval_all(&raw);
        self.vars.extend(hidden);
        let a = args?;
        let answer = match (name, a.as_slice()) {
            ("convert", [q, unit]) => units::convert(q, unit),
            ("siunits", [q]) => units::siunits(q),
            ("dimension", [q]) => units::dimension(q),
            ("quantity", [value, unit]) => units::make(value, unit),
            ("newunit", [name, q]) => units::define(name, q),
            ("removeunit", [name]) => units::forget(name),
            ("newunit", _) => Err(
                "newunit(\"name\", quantity), for example newunit(\"fortnight\", 14 day)".into(),
            ),
            ("removeunit", _) => {
                Err("removeunit(\"name\"): the name of a unit that newunit made".into())
            }
            ("convert", _) => {
                Err("convert(quantity, unit), for example convert(100 km/h, m/s)".into())
            }
            _ => Err(format!(
                "{name} takes one quantity, for example {name}(60 mph)"
            )),
        }?;
        for note in answer.notes {
            if !self.notes.contains(&note) {
                self.notes.push(note);
            }
        }
        Ok(answer.value)
    }

    /// The arguments of a command that draws, as they were written, apart from what is chosen
    /// for its picture there and in the arguments `more` of the command that saves it.
    pub(crate) fn chosen(
        &mut self,
        name: &str,
        raw: &[Expr],
        more: &[Expr],
    ) -> Result<(Vec<Expr>, crate::picture::Choices), String> {
        let mut choices = crate::picture::Choices::default();
        let mut plain = Vec::new();
        for (k, arg) in raw.iter().chain(more).enumerate() {
            // rotate(yaw, pitch, roll) after a drawing, or in plot4d: its body is turned
            if let Expr::Fn(f, angles) = arg
                && &**f == "rotate"
                && (1..=3).contains(&angles.len())
                && (k >= raw.len() || matches!(name, "plot4d" | "view4d"))
            {
                let angles = self.eval_all(angles)?;
                if angles.iter().all(|v| crate::plot::number(v).is_some()) {
                    choices.set("rotate", Some(&Expr::list(angles)))?;
                    continue;
                }
            }
            if matches!(arg, Expr::Fn(f, _) if &**f == "rotate")
                && (k >= raw.len() || matches!(name, "plot4d" | "view4d"))
            {
                return Err(format!(
                    "rotate(yaw, pitch, roll) takes one to three numbers: the degrees that the body of plot4d is turned about its upright axis, about its width and about its frames; not {arg}"
                ));
            }
            match crate::picture::choice(arg) {
                Some((option, value)) => {
                    let value = match value {
                        Some(v) => Some(self.eval(v)?),
                        None => None,
                    };
                    choices.set(option, value.as_ref())?;
                }
                None if k >= raw.len() => {
                    return Err(crate::picture::stray(arg).unwrap_or_else(|| {
                        format!("{arg} is nothing that is chosen for a picture: after the drawing come width = 800, gray and the like")
                    }));
                }
                None => {
                    // (a name that is no choice is a slip of the pen where no equation stands)
                    if (crate::walks::is_walk(name) || matches!(name, "plot4d" | "view4d"))
                        && matches!(arg, Expr::Rel(crate::expr::Rel::Eq, l, _) if l.sym_name().is_some())
                        && let Some(why) = crate::picture::stray(arg)
                    {
                        return Err(why);
                    }
                    plain.push(arg.clone());
                }
            }
        }
        Ok((plain, choices))
    }

    /// The drawing of a command with what is chosen for its picture, in its own arguments
    /// and in those of the command that saves it. `pixels`: for a picture of pixels.
    fn drawing(
        &mut self,
        name: &str,
        raw: &[Expr],
        more: &[Expr],
        pixels: bool,
        fast: bool,
    ) -> Result<(crate::scene::Drawing, crate::picture::Choices), String> {
        let (plain, mut choices) = self.chosen(name, raw, more)?;
        choices.fast = fast;
        let args = self.eval_args(&plain)?;
        let args = match crate::morecharts::is_control_chart(name) {
            true => self.control_chart(name, &args)?,
            false => args,
        };
        // (a chart of a grid of the earth is the chart of its matrix)
        let (name, args) = match self.map_chart(name, &args)? {
            Some((other, args)) => (other, args),
            None => (name, args),
        };
        let mut notes = Vec::new();
        let drawing = if name == "wire4d" {
            crate::solids4d::wire(&args, choices.look.frames)?
        } else if crate::walks::is_walk(name) {
            crate::walks::build(name, &args, &mut notes)?
        } else {
            crate::chart::build(name, &args, choices.detail(pixels))?
        };
        for note in notes {
            if !self.notes.contains(&note) {
                self.notes.push(note);
            }
        }
        Ok((drawing, choices))
    }

    /// `save_png("name.png", drawing, choices)` and its kin.
    fn save(&mut self, name: &str, raw: &[Expr]) -> Result<Vec<String>, String> {
        let usage = || {
            format!(
                "{name}(\"name\", drawing, choices): {name}(\"walk.{}\", walk(pi, 10, 1000), width = 800); the drawing is one of walk, walk3d, primewalk, primewalk3d, primemodwalk, primemodwalk3d, ulamspiral, sacksspiral, plot, parametricplot, polarplot, listplot, histogram, implicitplot, complexplot, densityplot, plot3d, parametricplot3d, graphplot, matrixplot, lineplot, scatter, bar, barh, stairs, stem, errorbar, pie, loglog, semilogx, semilogy, contour, quiver, plot3, scatter3, surf, spy, boxplot, stem3, quiver3, contour3, waterfall, compass, feather, polarscatter, bar3, histogram2, surfc, streamline, polarhistogram, dendrogram, bubblechart, fimplicit3, isosurface, triplot, trisurf, pareto, fill, ribbon, normplot, parallelplot, swarmchart, violinplot, histfit, gscatter, andrewsplot, bar3h, cdfplot, plotmatrix, stackedplot, biplot, bodeplot, nyquistplot, pzplot, rlocusplot, stepplot, impulseplot, zplane, plot4d, view4d; {name}(drawing) takes the name of the drawing for the file",
                if name.starts_with("save_jxl") {
                    "jxl"
                } else {
                    "png"
                }
            )
        };
        let fast = name.ends_with("_fast");
        let known = |e: &Expr| {
            matches!(e, Expr::Fn(command, _)
                if crate::picture::is_drawing(command) || crate::plot::is_plot(command))
        };
        let (path, drawn, more) = match raw {
            // a drawing without a name is saved under its own
            [drawn @ Expr::Fn(command, _), more @ ..]
                if known(drawn) && !name.starts_with("save_fast") && name != "save" =>
            {
                let end = if name.starts_with("save_jxl") {
                    "jxl"
                } else {
                    "png"
                };
                (crate::texts::text(&format!("{command}.{end}")), drawn, more)
            }
            [path, drawn, more @ ..] => (self.eval(path)?, drawn, more),
            _ => return Err(usage()),
        };
        let path = crate::radix::text_of(&path)
            .ok_or_else(|| {
                format!(
                    "the name of the file is a text in quotes, not {path}: {}",
                    usage()
                )
            })?
            .to_string();
        let format = crate::picture::format_of(name, &path)?;
        let Expr::Fn(command, args) = drawn else {
            return Err(usage());
        };
        if !crate::picture::is_drawing(command) && !crate::plot::is_plot(command) {
            return Err(usage());
        }
        if &**command == "plot4d" {
            return self.plot4d(args, more, Some((&path, format, fast)));
        }
        if &**command == "view4d" {
            let (drawing, choices) = self.view4d(args, more, true, fast)?;
            return crate::picture::save(&path, format, &drawing, &choices);
        }
        let (drawing, choices) = self.drawing(command, args, more, true, fast)?;
        crate::picture::save(&path, format, &drawing, &choices)
    }

    /// `base_conv(number, from, to)`: the first argument is written in the base `from`, as a
    /// number, or as an expression whose numbers are written in that base.
    fn base_conv(&mut self, raw: &[Expr]) -> Res {
        let from = self.eval(&raw[1])?;
        let from = crate::radix::base_of(Some(&from))?;
        let mut notes = Vec::new();
        let value = match crate::radix::text_of(&raw[0]) {
            Some(text) => match crate::radix::read(text, from, &mut notes) {
                Ok(number) => Expr::rational(number),
                Err(why) => {
                    let parsed = {
                        let funcs = &self.funcs;
                        parser::parse_radix(text, from, &|name| funcs.contains_key(name))
                    };
                    // (a number that names its base says itself what is wrong with it)
                    let named = text
                        .trim_start_matches([' ', '-', '+'])
                        .trim_start_matches(|c: char| c.is_ascii_digit())
                        .starts_with('#')
                        && text.contains(|c: char| c.is_ascii_digit());
                    let value = match parsed {
                        Ok(p) if p.stmts.len() == 1 => self.eval(&p.stmts[0]).ok(),
                        Err(reason) if named && !text.trim_start().starts_with('#') => {
                            return Err(reason);
                        }
                        _ => None,
                    };
                    let number = |v: &Expr| {
                        let v = crate::radix::plain(v.clone());
                        v.is_const()
                            && !v.is_infinite()
                            && !matches!(v, Expr::Rel(..) | Expr::Bool(_))
                    };
                    value.filter(number).ok_or(why)?
                }
            },
            None => self.eval(&raw[0])?,
        };
        let mut args = vec![value];
        for e in &raw[2..] {
            args.push(self.eval(e)?);
        }
        let (value, more) = crate::radix::call("tobase", &args)
            .ok_or("base_conv(number, from, to) or base_conv(value, to)")??;
        for note in notes.into_iter().chain(more) {
            if !self.notes.contains(&note) {
                self.notes.push(note);
            }
        }
        Ok(value)
    }

    /// Operands: a number that was written in a base counts as its value.
    pub(crate) fn eval_all(&mut self, items: &[Expr]) -> Result<Vec<Expr>, String> {
        items
            .iter()
            .map(|x| self.eval(x).map(crate::radix::plain))
            .collect()
    }

    /// The arguments of a command; `j` and `k` named as its variable are not quaternion units
    /// inside it.
    pub(crate) fn eval_args(&mut self, raw: &[Expr]) -> Result<Vec<Expr>, String> {
        let mut bound: Vec<String> = Vec::new();
        if assume::quaternion_mode() {
            for arg in raw.iter().skip(1) {
                let names: Vec<&Expr> = match arg {
                    Expr::List(items) => items.iter().collect(),
                    other => vec![other],
                };
                for v in names.into_iter().filter_map(Expr::sym_name) {
                    if matches!(v, "j" | "k") && !self.vars.contains_key(v) {
                        bound.push(v.to_string());
                    }
                }
            }
        }
        for v in &bound {
            self.vars.insert(v.clone(), Expr::sym(v));
        }
        let evaluated = self.eval_all(raw);
        for v in &bound {
            self.vars.remove(v);
        }
        evaluated
    }

    pub fn eval(&mut self, e: &Expr) -> Res {
        match e {
            Expr::Num(_) | Expr::Float(_) | Expr::Bool(_) => Ok(e.clone()),
            Expr::Sym(s) => match self.vars.get(&**s) {
                Some(v) => {
                    crate::trace::detail(|| format!("{s} stands for {v}"));
                    Ok(v.clone())
                }
                None if &**s == "ans" => self
                    .ans
                    .clone()
                    .ok_or_else(|| "there is no previous result".to_string()),
                None if assume::quaternion_mode() && matches!(&**s, "j" | "k") => {
                    let unit = |on: bool| if on { Expr::one() } else { Expr::zero() };
                    Ok(Expr::call(
                        "quat",
                        vec![
                            unit(false),
                            unit(false),
                            unit(&**s == "j"),
                            unit(&**s == "k"),
                        ],
                    ))
                }
                None => Ok(e.clone()),
            },
            // (systems of control are joined by the operations as numbers are)
            Expr::Add(a) => {
                let items = self.eval_all(a)?;
                self.joined(true, items)
            }
            Expr::Mul(a) => {
                let items = self.eval_all(a)?;
                self.joined(false, items)
            }
            Expr::Pow(b, x) => {
                let base = crate::radix::plain(self.eval(b)?);
                let exponent = crate::radix::plain(self.eval(x)?);
                self.raised(base, exponent)
            }
            Expr::List(a) => Ok(Expr::list(
                a.iter().map(|x| self.eval(x)).collect::<Result<_, _>>()?,
            )),
            Expr::Rel(op, a, b) => {
                let left = crate::radix::plain(self.eval(a)?);
                Ok(Expr::rel(*op, left, crate::radix::plain(self.eval(b)?)))
            }
            // (what a function makes of systems in a list, as sum does, is joined too)
            Expr::Fn(name, args) => match self.call(name, args)? {
                out if crate::control::joins_a_system(&out) => self.eval(&out),
                out => {
                    if crate::trace::deep() && !matches!(&**name, ":=" | "string" | "rule") {
                        let (asked, given) = (e.to_string(), out.to_string());
                        if asked != given {
                            crate::trace::detail(|| format!("{asked} gives {given}"));
                        }
                    }
                    Ok(out)
                }
            },
        }
    }

    fn define(&mut self, target: &Expr, body: &Expr) -> Res {
        match target {
            Expr::Sym(name) => {
                if crate::names::constant(name).is_some() || crate::names::function(name).is_some()
                {
                    return Err(format!("`{name}` is a reserved name"));
                }
                if let Some((params, inner)) = crate::lists::lambda(body) {
                    // f := x -> expr is f(x) := expr
                    self.vars.remove(&**name);
                    self.funcs
                        .insert(name.to_string(), (params.clone(), inner.clone()));
                    let head = Expr::call(name, params.iter().map(|p| Expr::sym(p)).collect());
                    return Ok(Expr::call(":=", vec![head, inner.clone()]));
                }
                let value = self.eval(body)?;
                if let Some((params, inner)) = crate::lists::lambda(&value) {
                    // (a function that was worked out, as matlabfunction gives one)
                    self.vars.remove(&**name);
                    self.funcs
                        .insert(name.to_string(), (params.clone(), inner.clone()));
                    let head = Expr::call(name, params.iter().map(|p| Expr::sym(p)).collect());
                    return Ok(Expr::call(":=", vec![head, inner.clone()]));
                }
                self.vars.insert(name.to_string(), value.clone());
                Ok(value)
            }
            Expr::Fn(name, params) => {
                if crate::names::function(name).is_some() {
                    return Err(format!("`{name}` is a built-in function"));
                }
                let params: Vec<String> = params
                    .iter()
                    .filter_map(|p| p.sym_name().map(str::to_string))
                    .collect();
                self.funcs.insert(name.to_string(), (params, body.clone()));
                Ok(Expr::call(":=", vec![target.clone(), body.clone()]))
            }
            _ => Err("only a name or f(x, ...) can be defined".into()),
        }
    }

    pub(crate) fn apply_user(&mut self, name: &str, args: &[Expr]) -> Res {
        if too_deep(self.calls.len()) {
            let itself = self.calls_itself(name);
            return Err(format!("{itself} calls itself without end"));
        }
        self.calls.push(name.to_string());
        let result = self.apply_user_body(name, args);
        self.calls.pop();
        result
    }

    /// Of the functions whose calls are under way when `name` is called once too often, the
    /// first that is called again further in: the same one however deep the calls could go,
    /// which is not the same on every system.
    fn calls_itself(&self, name: &str) -> String {
        let mut times: HashMap<&str, u32> = HashMap::new();
        for call in self.calls.iter().map(String::as_str).chain([name]) {
            *times.entry(call).or_default() += 1;
        }
        let again = self.calls.iter().find(|call| times[call.as_str()] > 1);
        again.map_or(name, String::as_str).to_string()
    }

    fn apply_user_body(&mut self, name: &str, args: &[Expr]) -> Res {
        let (params, body) = self.funcs[name].clone();
        if params.len() != args.len() {
            return Err(format!(
                "{name} takes {} argument(s), {} given",
                params.len(),
                args.len()
            ));
        }
        let values = self.eval_all(args)?;
        let saved: Vec<Option<Expr>> = params.iter().map(|p| self.vars.get(p).cloned()).collect();
        for (p, v) in params.iter().zip(values) {
            self.vars.insert(p.clone(), v);
        }
        let result = self.eval(&body);
        for (p, old) in params.iter().zip(saved) {
            match old {
                Some(v) => self.vars.insert(p.clone(), v),
                None => self.vars.remove(p),
            };
        }
        result
    }

    pub(crate) fn call(&mut self, name: &str, raw: &[Expr]) -> Res {
        if name == "clear" {
            return self.clear(raw);
        }
        match name {
            ":=" => return self.define(&raw[0], &raw[1]),
            "rule"
                if raw.len() == 2
                    && crate::lists::lambda(&Expr::call("rule", raw.to_vec())).is_some() =>
            {
                // x -> body: the body is worked out with x as a plain name
                let (params, _) =
                    crate::lists::lambda(&Expr::call("rule", raw.to_vec())).expect("checked");
                let saved: Vec<Option<Expr>> = params.iter().map(|p| self.vars.remove(p)).collect();
                let body = self.eval(&raw[1]);
                for (p, old) in params.iter().zip(saved) {
                    if let Some(v) = old {
                        self.vars.insert(p.clone(), v);
                    }
                }
                return Ok(Expr::call("rule", vec![raw[0].clone(), body?]));
            }
            "assume" => return self.assume(raw),
            "forget" => {
                match raw.first().and_then(Expr::sym_name) {
                    Some(v) => assume::clear(v),
                    None => assume::clear_all(),
                }
                return Ok(Expr::Bool(true));
            }
            "domain" => {
                use assume::Domain;
                let domain = match raw.first().map(|d| d.sym_name()) {
                    None => assume::domain(),
                    Some(Some("real" | "reals" | "R")) => Domain::Real,
                    Some(Some("complex" | "complexes" | "C")) => Domain::Complex,
                    Some(Some("quaternion" | "quaternions" | "H")) => Domain::Quaternion,
                    Some(_) => {
                        return Err(
                            "domain(complex), domain(real) or domain(quaternion)".to_string()
                        );
                    }
                };
                assume::set_domain(domain);
                return Ok(Expr::sym(match domain {
                    Domain::Complex => "complex",
                    Domain::Real => "real",
                    Domain::Quaternion => "quaternion",
                }));
            }
            "dsolve" | "rsolve" | "funsolve" | "ndsolve" | "qsolve" => {
                return Err(format!("{name} must be a statement of its own"));
            }
            "string" => return Ok(Expr::call(name, raw.to_vec())),
            "base_conv" if matches!(raw.len(), 3 | 4) => return self.base_conv(raw),
            _ if crate::plot::is_plot(name) => {
                return Err(format!("{name} must be a statement of its own"));
            }
            _ if crate::picture::is_save(name) => {
                return Err(format!("{name} must be a statement of its own"));
            }
            _ if crate::picture::is_drawing(name) => {
                return Err(format!(
                    "{name} must be a statement of its own, or the drawing of save_png(\"name.png\", {name}(...))"
                ));
            }
            "derivative'" => {
                // f'(x) for a defined function: the derivative of its body
                let (mut depth, mut inner) = (1u32, &raw[0]);
                while let Expr::Fn(n, a) = inner
                    && &**n == "derivative'"
                {
                    depth += 1;
                    inner = &a[0];
                }
                if let Expr::Fn(f, args) = inner
                    && let Some((params, body)) = self.funcs.get(&**f).cloned()
                    && params.len() == 1
                    && args.len() == 1
                {
                    let saved = self.vars.remove(&params[0]);
                    let value = self.eval(&body);
                    if let Some(v) = saved {
                        self.vars.insert(params[0].clone(), v);
                    }
                    let derived = crate::diff::diff_n(&value?, &params[0], depth);
                    let at = self.eval(&args[0])?;
                    return Ok(simplify(&subst(&derived, &Expr::sym(&params[0]), &at)));
                }
            }
            _ if self.funcs.contains_key(name) => return self.apply_user(name, raw),
            _ => {}
        }
        if let Some(r) = self.form(name, raw) {
            return r;
        }
        if crate::units::takes_units(name) {
            return self.units(name, raw);
        }
        let a = if parser::FUNCTION_ARGS.contains(&name) {
            // a function that is written out stays as it is written: it is worked out when
            // it gets its arguments, which may be lists
            let written = |r: &Expr| crate::lists::lambda(r).is_some();
            let staged: Vec<Expr> = raw
                .iter()
                .map(|r| if written(r) { Expr::zero() } else { r.clone() })
                .collect();
            let mut a = self.eval_args(&staged)?;
            for (value, r) in a.iter_mut().zip(raw) {
                if written(r) {
                    *value = r.clone();
                }
            }
            a
        } else {
            self.eval_args(raw)?
        };
        if matches!(
            name,
            "diff" | "integrate" | "limit" | "sum" | "product" | "series" | "N"
        ) && let Some(parts) = a.first().and_then(crate::quat::parts)
        {
            let mut out = Vec::new();
            for p in parts {
                let mut args = a.clone();
                args[0] = p;
                out.push(self.call(name, &args)?);
            }
            let [x, y, z, w] = <[Expr; 4]>::try_from(out).expect("four components");
            return Ok(crate::quat::make([x, y, z, w]));
        }
        let digits = numeric::digits();
        if let Some(r) = self.cells(name, &a) {
            return r;
        }
        if let Some(r) = self.delays(name, &a) {
            return r;
        }
        if let Some(r) = self.mimo(name, &a) {
            return r;
        }
        if let Some(parts) = crate::sigdesign::apart(name, &a) {
            return self.call(name, &parts?);
        }
        if let Some(r) = self.control(name, &a) {
            return r;
        }
        if let Some(r) = self.sigdesign(name, &a) {
            return r;
        }
        if let Some(r) = self.sigmore(name, &a) {
            return r;
        }
        if let Some(r) = crate::geodesy::call(name, &a) {
            return r;
        }
        if let Some(r) = self.georaster(name, &a) {
            return r;
        }
        if let Some(r) = crate::solids4d::call(name, &a) {
            return r;
        }
        if let Some(r) = crate::neural::call(name, &a) {
            return r;
        }
        if let Some(r) = self.table_function(name, &a) {
            return r;
        }
        if let Some(r) = self.table_call(name, &a) {
            return r;
        }
        if let Some(r) = self.matlab(name, &a) {
            return r;
        }
        if let Some(r) = self.hypotest(name, &a) {
            return r;
        }
        if let Some(r) = self.solver(name, &a) {
            return r;
        }
        if let Some(r) = self.code_call(name, &a) {
            return r;
        }
        if let Some(r) = self.record_call(name, &a) {
            return r;
        }
        if let Some(r) = crate::texts::call(name, &a)
            .or_else(|| crate::arrays::call(name, &a))
            .or_else(|| crate::datafun::call(name, &a))
            .or_else(|| crate::decomp::call(name, &a))
            .or_else(|| crate::polys::call(name, &a))
            .or_else(|| crate::signal::call(name, &a))
            .or_else(|| crate::elem::call(name, &a))
            .or_else(|| crate::random::call(name, &a))
            .or_else(|| crate::matfile::call(name, &a))
            .or_else(|| crate::blocks::call(name, &a))
            .or_else(|| crate::imageproc::call(name, &a))
            .or_else(|| crate::optim::call(name, &a))
            .or_else(|| crate::compgeom::call(name, &a))
            .or_else(|| crate::extras::call(name, &a))
            .or_else(|| crate::imagefun::call(name, &a))
            .or_else(|| crate::dates::call(name, &a))
            .or_else(|| crate::patterns::call(name, &a))
            .or_else(|| crate::gallery::call(name, &a))
            .or_else(|| crate::pencils::call(name, &a))
            .or_else(|| crate::multivar::call(name, &a))
        {
            return r;
        }
        if let Some(r) = crate::graph::call(name, &a) {
            return r;
        }
        if let Some(r) = crate::radix::call(name, &a) {
            let (value, notes) = r?;
            for note in notes {
                if !self.notes.contains(&note) {
                    self.notes.push(note);
                }
            }
            return Ok(value);
        }
        if let Some(r) = self.distribution(name, &a) {
            return r;
        }
        if let Some(r) = crate::dist::construct(name, &a) {
            return r;
        }
        Ok(match (name, a.len()) {
            ("hold", 1) => a[0].clone(),
            ("N", 1) => numeric::n(&a[0], digits),
            ("N", 2) => {
                let d = a[1]
                    .as_i64()
                    .filter(|d| (1..=1_000_000).contains(d))
                    .ok_or("N(expr, digits): digits must be a positive whole number")?;
                numeric::n(&a[0], d as u32)
            }
            ("simplify", 1) => simplify(&a[0]),
            ("expand", 1) => expand(&a[0]),
            ("rewrite_exp", 1) => crate::solve::rewrite_exp(&a[0]),
            ("factor", 1) => crate::ratsimp::factor(&a[0]),
            ("together" | "cancel", 1) => crate::ratsimp::together(&a[0]),
            ("numer", 1) => crate::ratsimp::fraction(&a[0]).0,
            ("denom", 1) => crate::ratsimp::fraction(&a[0]).1,
            ("subst", 2 | 3) => self.subst(&a)?,
            ("rectform" | "complexexpand", 1) => {
                let all_real = name == "complexexpand";
                let (written, parts) = if all_real {
                    complex::complex_expand(&a[0])
                } else {
                    (a[0].clone(), complex::rect(&a[0]))
                };
                let tidy = |e: &Expr| {
                    if all_real {
                        assume::with_real_symbols(|| simplify(e))
                    } else {
                        simplify(e)
                    }
                };
                match parts {
                    Some((re, im)) => {
                        let (re, im) = (tidy(&re), tidy(&im));
                        let turned = mul(vec![Expr::i(), im.clone()]);
                        if im.is_zero() {
                            re
                        } else if re.is_zero() {
                            turned
                        } else {
                            let mut terms = match &re {
                                Expr::Add(t) => t.to_vec(),
                                _ => vec![re],
                            };
                            terms.push(match &im {
                                Expr::Add(_) => Expr::Mul(vec![Expr::i(), im].into()),
                                _ => turned,
                            });
                            Expr::call("ordered", terms)
                        }
                    }
                    None => written,
                }
            }
            ("polarform", 1) => {
                let (r, t) = (
                    simplify(&func("abs", vec![a[0].clone()])),
                    func("arg", vec![a[0].clone()]),
                );
                let turn = Expr::Pow(Expr::e().into(), mul(vec![Expr::i(), t]).into());
                if r.is_zero() {
                    return Ok(r);
                }
                Expr::call(
                    "hold",
                    vec![if r.is_one() {
                        turn
                    } else {
                        Expr::Mul(vec![r, turn].into())
                    }],
                )
            }
            ("inv", 1) => match matrix::rows(&a[0]) {
                Some(m) => matrix::from_rows(matrix::inverse(&m)?),
                None => matrix::pow(a[0].clone(), Expr::minus_one())?,
            },
            ("norm", 1) => match a[0].as_list() {
                Some(v) => canon::sqrt(add(v
                    .iter()
                    .map(|x| pow(func("abs", vec![x.clone()]), Expr::int(2)))
                    .collect())),
                None => func("abs", vec![a[0].clone()]),
            },
            ("avg", n) => {
                let items: Vec<Expr> = if n == 1 && a[0].as_list().is_some() {
                    a[0].as_list().unwrap().to_vec()
                } else {
                    a
                };
                let count = items.len() as i64;
                div(add(items), Expr::int(count))
            }
            ("minabs" | "maxabs", 2) => {
                let d = numeric::sign(
                    &(func("abs", vec![a[0].clone()]) - func("abs", vec![a[1].clone()])),
                );
                match d {
                    Some(s) if (s <= 0) == (name == "minabs") => a[0].clone(),
                    Some(_) => a[1].clone(),
                    None => Expr::call(name, a),
                }
            }
            ("is_nan", 1) => Expr::int(i64::from(a[0].is_undefined())),
            ("is_infinite", 1) => Expr::int(i64::from(a[0].is_infinite() && !a[0].is_undefined())),
            ("is_finite", 1) => Expr::int(i64::from(!a[0].is_infinite())),
            ("is_normal", 1) => Expr::int(i64::from(!a[0].is_infinite() && !a[0].is_zero())),
            ("det", 1) => matrix::det(&matrix::rows(&a[0]).ok_or("det needs a matrix")?)?,
            ("transpose", 1) => matrix::from_rows(matrix::transpose(
                &matrix::rows(&a[0]).ok_or("transpose needs a matrix")?,
            )),
            ("rule" | "element" | "onesided", _) => Expr::call(name, a),
            _ if crate::region::is_region(&Expr::call(name, Vec::new())) => Expr::call(name, a),
            ("ordered", _) => add(a),
            _ => match self.spatial(name, &a).or_else(|| self.calculus(name, &a)) {
                Some(r) => r?,
                None => match self
                    .commands(name, &a)
                    .or_else(|| crate::lists::pure(name, &a))
                    .or_else(|| crate::stats::call(name, &a))
                {
                    Some(r) => r?,
                    None => listable(name, a),
                },
            },
        })
    }

    /// Variable of a command: the one given, else the preferred free symbol of the expression.
    fn var_of(given: Option<&Expr>, e: &Expr) -> Result<String, String> {
        match given {
            Some(Expr::Sym(s)) => Ok(s.to_string()),
            Some(other) => Err(format!("{other} is not a variable")),
            None => pick_vars(std::slice::from_ref(e), 1)
                .pop()
                .ok_or_else(|| "the expression has no variable".to_string()),
        }
    }

    fn var_list(given: Option<&Expr>, e: &Expr) -> Result<Vec<String>, String> {
        match given {
            Some(Expr::List(items)) => items.iter().map(|i| Self::var_of(Some(i), e)).collect(),
            Some(other) => Ok(vec![Self::var_of(Some(other), e)?]),
            None => Ok(pick_vars(std::slice::from_ref(e), usize::MAX)),
        }
    }

    /// Tensors, geometry and more of linear algebra.
    fn spatial(&mut self, name: &str, a: &[Expr]) -> Option<Res> {
        use crate::geometry as geo;
        use crate::tensor::{self, Metric, Tensor};
        let index = |e: &Expr, what: &str| {
            e.as_i64()
                .filter(|k| *k >= 1)
                .map(|k| k as usize - 1)
                .ok_or_else(|| format!("{what} must be a whole number from 1 on"))
        };
        let names = |e: &Expr| -> Result<Vec<String>, String> {
            e.as_list()
                .and_then(|v| v.iter().map(|x| x.sym_name().map(str::to_string)).collect())
                .ok_or_else(|| format!("{e} is not a list of coordinates"))
        };
        let metric = |g: &Expr, x: &Expr| Metric::new(g, &names(x)?);
        let rows =
            |e: &Expr, what: &str| matrix::rows(e).ok_or_else(|| format!("{what} needs a matrix"));
        let points = |list: &[Expr]| list.iter().map(geo::point).collect::<Result<Vec<_>, _>>();
        let matrix_out = |m: Result<matrix::Rows, String>| m.map(matrix::from_rows);
        Some(match (name, a.len()) {
            ("dimensions", 1) => Ok(Expr::list(
                Tensor::of(&a[0])
                    .dims
                    .iter()
                    .map(|d| Expr::int(*d as i64))
                    .collect(),
            )),
            ("tensorrank", 1) => Ok(Expr::int(Tensor::of(&a[0]).rank() as i64)),
            ("flatten", 1) => {
                fn pour(e: &Expr, out: &mut Vec<Expr>) {
                    match e {
                        Expr::List(items) => items.iter().for_each(|i| pour(i, out)),
                        other => out.push(other.clone()),
                    }
                }
                let mut out = Vec::new();
                pour(&a[0], &mut out);
                Ok(Expr::list(out))
            }
            ("reshape", 2) => (|| {
                let dims: Vec<usize> = a[1]
                    .as_list()
                    .and_then(|d| {
                        d.iter()
                            .map(|k| k.as_i64().filter(|k| *k > 0).map(|k| k as usize))
                            .collect()
                    })
                    .ok_or("reshape(list, [n1, n2, ...]) needs whole sizes")?;
                let data = Tensor::of(&a[0]).data;
                if dims.iter().product::<usize>() != data.len() {
                    return Err(format!("{} entries do not fill these sizes", data.len()));
                }
                Ok(Tensor { dims, data }.expr())
            })(),
            ("transpose", 2) => (|| {
                let order: Vec<usize> = a[1]
                    .as_list()
                    .ok_or("transpose(t, [2, 1, 3]) needs the order of the indices")?
                    .iter()
                    .map(|k| index(k, "an index"))
                    .collect::<Result<_, _>>()?;
                Ok(Tensor::of(&a[0]).transpose(&order)?.expr())
            })(),
            ("outer", n) if n >= 2 => Ok(a[1..]
                .iter()
                .fold(Tensor::of(&a[0]), |t, x| t.outer(&Tensor::of(x)))
                .expr()),
            ("contract", 3) => (|| {
                let (i, j) = (index(&a[1], "an index")?, index(&a[2], "an index")?);
                Ok(simplify(&Tensor::of(&a[0]).contract(i, j)?.expr()))
            })(),
            ("inner", 2 | 3) => (|| {
                let count = match a.get(2) {
                    Some(k) => index(k, "the number of indices")? + 1,
                    None => 1,
                };
                Ok(simplify(
                    &Tensor::of(&a[0]).inner(&Tensor::of(&a[1]), count)?.expr(),
                ))
            })(),
            ("kron", 2) => (|| {
                Ok(matrix::from_rows(tensor::kron(
                    &rows(&a[0], "kron")?,
                    &rows(&a[1], "kron")?,
                )))
            })(),
            ("levicivita", 1) if a[0].as_list().is_none() => a[0]
                .as_i64()
                .filter(|n| (1..=6).contains(n))
                .map(|n| tensor::levi_civita(n as usize).expr())
                .ok_or_else(|| "levicivita(n) needs a dimension from 1 to 6".to_string()),
            ("levicivita" | "signature", _) => (|| {
                let items: &[Expr] = match a {
                    [Expr::List(v)] => v,
                    other => other,
                };
                let p: Vec<usize> = items
                    .iter()
                    .map(|k| k.as_i64().filter(|k| *k >= 0).map(|k| k as usize))
                    .collect::<Option<_>>()
                    .ok_or("the entries must be whole numbers")?;
                Ok(Expr::int(tensor::signature(&p)))
            })(),
            ("kroneckerdelta", n) if n >= 2 => Ok(
                if a.windows(2)
                    .all(|w| crate::simplify::is_zero(&canon::sub(w[0].clone(), w[1].clone())))
                {
                    Expr::one()
                } else if a.iter().all(Expr::is_const) {
                    Expr::zero()
                } else {
                    Expr::call(name, a.to_vec())
                },
            ),
            ("symmetrize" | "antisymmetrize", 1) => Tensor::of(&a[0])
                .symmetrize(name == "antisymmetrize")
                .map(|t| t.expr()),
            ("wedge", 2) => tensor::wedge(&Tensor::of(&a[0]), &Tensor::of(&a[1])).map(|t| t.expr()),
            ("hodge", 1) => tensor::hodge(&Tensor::of(&a[0])).map(|t| t.expr()),
            ("christoffel" | "riemann" | "ricci" | "einstein", 2) => {
                metric(&a[0], &a[1]).map(|m| {
                    match name {
                        "christoffel" => m.christoffel(),
                        "riemann" => m.riemann(),
                        "ricci" => m.ricci(),
                        _ => m.einstein(),
                    }
                    .expr()
                })
            }
            ("ricciscalar", 2) => metric(&a[0], &a[1]).map(|m| m.scalar()),
            ("kretschmann", 2) => metric(&a[0], &a[1]).and_then(|m| m.kretschmann()),
            ("geodesics", 2) => (|| {
                let m = metric(&a[0], &a[1])?;
                let x = names(&a[1])?;
                let prime = |e: Expr| Expr::call("derivative'", vec![e]);
                let v: Vec<Expr> = x.iter().map(|n| prime(Expr::sym(n))).collect();
                Ok(Expr::list(
                    m.geodesics(&v)
                        .into_iter()
                        .zip(&v)
                        .map(|(right, first)| Expr::call("rule", vec![prime(first.clone()), right]))
                        .collect(),
                ))
            })(),
            ("covariant", 3) => (|| {
                let v = a[0]
                    .as_list()
                    .ok_or("covariant(v, g, [x, ...]) needs a vector field")?;
                Ok(metric(&a[1], &a[2])?.covariant(v)?.expr())
            })(),
            ("raiseindex" | "lowerindex", 3) => (|| {
                let n = rows(&a[1], name)?.len();
                let dummy: Vec<String> = (0..n).map(|k| format!("#x{k}")).collect();
                let m = Metric::new(&a[1], &dummy)?;
                let k = index(&a[2], "the index")?;
                Ok(m.shift(&Tensor::of(&a[0]), k, name == "raiseindex")?.expr())
            })(),
            ("laplacian", 3) => metric(&a[2], &a[1]).and_then(|m| m.laplacian(&a[0])),
            ("divergence", 3) => (|| {
                let v = a[0]
                    .as_list()
                    .ok_or("divergence(v, [x, ...], g) needs a vector field")?;
                metric(&a[2], &a[1])?.divergence(v)
            })(),
            ("line" | "plane" | "circle" | "sphere", _) => Ok(Expr::call(name, a.to_vec())),
            ("distance", 2) => geo::distance(&a[0], &a[1]),
            ("angle", 2 | 3) => geo::angle(a),
            ("projection", 2) => geo::projection(&a[0], &a[1]),
            ("intersection", 2)
                if !a.iter().all(
                    |x| matches!(x, Expr::List(l) if !l.iter().any(|p| matches!(p, Expr::List(_)))),
                ) =>
            {
                geo::intersection(&a[0], &a[1])
            }
            ("equation", 1) => geo::equation(&a[0]),
            ("midpoint", 2) => (|| {
                let (p, q) = (geo::point(&a[0])?, geo::point(&a[1])?);
                if p.len() != q.len() {
                    return Err("the points have different numbers of coordinates".into());
                }
                Ok(Expr::list(
                    p.iter()
                        .zip(&q)
                        .map(|(x, y)| simplify(&div(add(vec![x.clone(), y.clone()]), Expr::int(2))))
                        .collect(),
                ))
            })(),
            ("centroid", n) if n > 1 || a[0].as_list().is_some() => (|| {
                let items: &[Expr] = match a {
                    [Expr::List(v)] => v,
                    other => other,
                };
                let ps = points(items)?;
                let size = ps.first().map_or(0, Vec::len);
                if size == 0 || ps.iter().any(|p| p.len() != size) {
                    return Err("centroid needs points with the same number of coordinates".into());
                }
                Ok(Expr::list(
                    (0..size)
                        .map(|k| {
                            simplify(&div(
                                add(ps.iter().map(|p| p[k].clone()).collect()),
                                Expr::int(ps.len() as i64),
                            ))
                        })
                        .collect(),
                ))
            })(),
            ("normalize", 1) if a[0].as_list().is_some() => (|| {
                let v = geo::point(&a[0])?;
                let size = canon::sqrt(add(v
                    .iter()
                    .map(|x| pow(x.clone(), Expr::int(2)))
                    .collect()));
                Ok(Expr::list(
                    v.iter()
                        .map(|x| simplify(&div(x.clone(), size.clone())))
                        .collect(),
                ))
            })(),
            ("norm", 2) => {
                let v = Tensor::of(&a[0]).data;
                let sizes: Vec<Expr> = v.iter().map(|x| func("abs", vec![x.clone()])).collect();
                Ok(if a[1].is_inf() {
                    func("max", sizes)
                } else {
                    simplify(&pow(
                        add(sizes.into_iter().map(|s| pow(s, a[1].clone())).collect()),
                        div(Expr::one(), a[1].clone()),
                    ))
                })
            }
            ("cross", n) if n != 2 || a[0].as_list().is_some_and(|v| v.len() != 3) => (|| {
                Ok(Expr::list(
                    geo::cross(&points(a)?)?.iter().map(simplify).collect(),
                ))
            })(
            ),
            ("triple", 3) => (|| Ok(simplify(&matrix::det(&points(a)?)?)))(),
            ("collinear" | "coplanar", n) if n >= 2 => (|| {
                let ps = points(a)?;
                let size = ps[0].len();
                if ps.iter().any(|p| p.len() != size) {
                    return Err("the points have different numbers of coordinates".into());
                }
                let edges: matrix::Rows = ps[1..]
                    .iter()
                    .map(|p| {
                        p.iter()
                            .zip(&ps[0])
                            .map(|(x, y)| canon::sub(x.clone(), y.clone()))
                            .collect()
                    })
                    .collect();
                let (_, rank, _) = matrix::eliminate(edges, false);
                Ok(Expr::Bool(rank <= if name == "collinear" { 1 } else { 2 }))
            })(),
            ("circumcircle" | "circumsphere", n) if n >= 2 => {
                points(a).and_then(|p| geo::circumsphere(&p))
            }
            ("incircle", 3) => points(a).and_then(|p| geo::incircle(&p[0], &p[1], &p[2])),
            ("solidangle", 1) => Ok(simplify(&mul(vec![
                Expr::int(2),
                Expr::pi(),
                canon::sub(Expr::one(), func("cos", vec![a[0].clone()])),
            ]))),
            ("solidangle", 3) => points(a).and_then(|p| geo::solid_angle(&p[0], &p[1], &p[2])),
            ("solidangle", 4) => points(a).and_then(|p| {
                let from = |q: &[Expr]| -> Vec<Expr> {
                    q.iter()
                        .zip(&p[0])
                        .map(|(x, y)| canon::sub(x.clone(), y.clone()))
                        .collect()
                };
                geo::solid_angle(&from(&p[1]), &from(&p[2]), &from(&p[3]))
            }),
            ("ballvolume" | "spherearea", 1 | 2) => Ok(geo::ball(
                &a[0],
                a.get(1).unwrap_or(&Expr::one()),
                name == "spherearea",
            )),
            ("rotationmatrix", 1) if crate::quat::is_quat(&a[0]) => {
                Ok(crate::quat::rotation_matrix(&a[0]))
            }
            ("rotationmatrix", 1..=3) => {
                matrix_out(points(&a[1..]).and_then(|frame| geo::rotation_matrix(&a[0], &frame)))
            }
            ("rotationquat", 2) => geo::point(&a[1]).and_then(|axis| {
                crate::quat::from_axis_angle(&axis, &a[0]).ok_or_else(|| {
                    "rotationquat(angle, [x, y, z]) needs an axis of space".to_string()
                })
            }),
            ("rotate", 2) if crate::quat::is_quat(&a[1]) => geo::point(&a[0]).and_then(|v| {
                crate::quat::rotate(&v, &a[1])
                    .ok_or_else(|| "rotate(v, q) needs a vector of space".to_string())
            }),
            ("rotate", 2 | 3) => (|| {
                let v = geo::point(&a[0])?;
                let frame = points(&a[2..])?;
                let m = geo::rotation_matrix(&a[1], &frame)?;
                if m.len() != v.len() {
                    return Err("the vector does not fit the rotation".into());
                }
                Ok(Expr::list(
                    m.iter()
                        .map(|r| {
                            simplify(&add(r
                                .iter()
                                .zip(&v)
                                .map(|(x, y)| mul(vec![x.clone(), y.clone()]))
                                .collect()))
                        })
                        .collect(),
                ))
            })(),
            ("reflect", 2) => (|| geo::reflect(&geo::point(&a[0])?, &geo::point(&a[1])?))(),
            ("topolar" | "tospherical", 1) => {
                geo::point(&a[0]).and_then(|p| geo::spherical(&p, true))
            }
            ("frompolar" | "fromspherical", 1) => {
                geo::point(&a[0]).and_then(|p| geo::spherical(&p, false))
            }
            ("regularpolygon", 1 | 2) => Ok(geo::regular_polygon(
                &a[0],
                a.get(1).unwrap_or(&Expr::sym("a")),
            )),
            ("polytope", 1..=3) => (|| {
                let kind = a[0]
                    .sym_name()
                    .ok_or("polytope(name) needs a name, as in polytope(tesseract)")?;
                let general = matches!(
                    kind,
                    "simplex" | "hypercube" | "orthoplex" | "crosspolytope"
                );
                let (n, edge) = if general {
                    (
                        Some(
                            a.get(1)
                                .and_then(Expr::as_i64)
                                .ok_or("polytope(simplex, n): the dimension n is missing")?,
                        ),
                        a.get(2),
                    )
                } else {
                    (None, a.get(1))
                };
                geo::polytope(kind, n, edge.unwrap_or(&Expr::sym("a")))
            })(),
            ("adjugate", 1) => (|| {
                let m = rows(&a[0], name)?;
                let n = m.len();
                if m.iter().any(|r| r.len() != n) {
                    return Err("adjugate needs a square matrix".into());
                }
                let minor = |i: usize, j: usize| -> Res {
                    let sub: matrix::Rows = m
                        .iter()
                        .enumerate()
                        .filter(|(r, _)| *r != i)
                        .map(|(_, row)| {
                            row.iter()
                                .enumerate()
                                .filter(|(c, _)| *c != j)
                                .map(|(_, x)| x.clone())
                                .collect()
                        })
                        .collect();
                    if sub.is_empty() {
                        return Ok(Expr::one());
                    }
                    let d = matrix::det(&sub)?;
                    Ok(simplify(&if (i + j).is_multiple_of(2) {
                        d
                    } else {
                        canon::neg(d)
                    }))
                };
                // the transpose of the matrix of cofactors
                let mut out = Vec::new();
                for i in 0..n {
                    let mut row = Vec::new();
                    for j in 0..n {
                        row.push(minor(j, i)?);
                    }
                    out.push(row);
                }
                Ok(matrix::from_rows(out))
            })(),
            ("matrixexp", 1 | 2) => (|| {
                if let Some(found) = crate::matfun::numeric(name, &rows(&a[0], name)?, a.get(1)) {
                    return found;
                }
                let z = Expr::sym("#w");
                let exponent = match a.get(1) {
                    Some(t) => mul(vec![t.clone(), z]),
                    None => z,
                };
                matrix_out(matrix::function_of(
                    &rows(&a[0], name)?,
                    &pow(Expr::e(), exponent),
                    "#w",
                ))
            })(),
            ("matrixpower", 2) => (|| {
                let m = rows(&a[0], name)?;
                if let Some(found) = crate::matfun::numeric(name, &m, a.get(1)) {
                    return found;
                }
                matrix_out(matrix::function_of(
                    &m,
                    &pow(Expr::sym("#w"), a[1].clone()),
                    "#w",
                ))
            })(),
            ("matrixsqrt" | "matrixlog", 1) => (|| {
                if let Some(found) = crate::matfun::numeric(name, &rows(&a[0], name)?, None) {
                    return found;
                }
                let f = if name == "matrixsqrt" {
                    canon::sqrt(Expr::sym("#w"))
                } else {
                    func("ln", vec![Expr::sym("#w")])
                };
                matrix_out(matrix::function_of(&rows(&a[0], name)?, &f, "#w"))
            })(),
            ("matrixfunction", 3) => (|| {
                let var = a[1].sym_name().ok_or("matrixfunction(f(x), x, A)")?;
                matrix_out(matrix::function_of(&rows(&a[2], name)?, &a[0], var))
            })(),
            ("gramschmidt", 1) => {
                matrix_out(rows(&a[0], name).and_then(|m| matrix::gram_schmidt(&m)))
            }
            ("cholesky", 1) => matrix_out(rows(&a[0], name).and_then(|m| matrix::cholesky(&m))),
            ("qr", 1) => (|| {
                let m = rows(&a[0], name)?;
                let columns = matrix::transpose(&m);
                let q: matrix::Rows = matrix::gram_schmidt(&columns)?
                    .into_iter()
                    .map(|v| {
                        let size = canon::sqrt(add(v
                            .iter()
                            .map(|x| pow(x.clone(), Expr::int(2)))
                            .collect()));
                        v.iter()
                            .map(|x| simplify(&div(x.clone(), size.clone())))
                            .collect()
                    })
                    .collect();
                if q.len() != columns.len() {
                    return Err("qr needs a matrix with independent columns".into());
                }
                let r = matrix::matmul(&q, &m)?;
                Ok(Expr::list(vec![
                    matrix::from_rows(matrix::transpose(&q)),
                    simplify(&matrix::from_rows(r)),
                ]))
            })(),
            ("pinv", 1) => (|| {
                let m = rows(&a[0], name)?;
                let t = matrix::transpose(&m);
                let tall = matrix::matmul(&t, &m)?;
                let out = match matrix::inverse(&tall) {
                    Ok(inv) => matrix::matmul(&inv, &t)?,
                    Err(_) => matrix::matmul(&t, &matrix::inverse(&matrix::matmul(&m, &t)?)?)?,
                };
                Ok(simplify(&matrix::from_rows(out)))
            })(),
            ("singularvalues", 1) => (|| {
                let m = rows(&a[0], name)?;
                let gram = matrix::matmul(&matrix::transpose(&m), &m)?;
                let z = Expr::sym("#z");
                let solved = crate::solve::solve(&matrix::charpoly(&gram, &z)?, "#z")?;
                crate::solve::release(&solved.params);
                let mut values: Vec<Expr> = solved
                    .roots
                    .iter()
                    .flat_map(|r| {
                        std::iter::repeat_n(
                            simplify(&canon::sqrt(r.value.clone())),
                            r.mult as usize,
                        )
                    })
                    .collect();
                values.sort_by(|x, y| {
                    let v = |e: &Expr| {
                        numeric::real_value(e, 20)
                            .map(|f| f.to_f64())
                            .unwrap_or(0.0)
                    };
                    v(y).total_cmp(&v(x))
                });
                Ok(Expr::list(values))
            })(),
            ("diag", 1) => (|| {
                Ok(match matrix::rows(&a[0]) {
                    Some(m) => Expr::list(
                        (0..m.len().min(m[0].len()))
                            .map(|k| m[k][k].clone())
                            .collect(),
                    ),
                    None => {
                        let d = a[0].as_list().ok_or("diag([a, b, ...]) or diag(matrix)")?;
                        matrix::from_rows(
                            (0..d.len())
                                .map(|i| {
                                    (0..d.len())
                                        .map(|j| if i == j { d[i].clone() } else { Expr::zero() })
                                        .collect()
                                })
                                .collect(),
                        )
                    }
                })
            })(),
            ("zeros" | "ones", 1 | 2) => (|| {
                let size = |e: &Expr| {
                    e.as_i64()
                        .filter(|n| (1..=1000).contains(n))
                        .map(|n| n as usize)
                        .ok_or("the sizes must be whole numbers from 1 to 1000")
                };
                let (r, c) = (size(&a[0])?, size(a.get(1).unwrap_or(&a[0]))?);
                let fill = if name == "zeros" {
                    Expr::zero()
                } else {
                    Expr::one()
                };
                Ok(matrix::from_rows(vec![vec![fill; c]; r]))
            })(),
            ("hilbert", 1) => a[0]
                .as_i64()
                .filter(|n| (1..=200).contains(n))
                .map(|n| {
                    matrix::from_rows(
                        (0..n)
                            .map(|i| (0..n).map(|j| Expr::rat(1, i + j + 1)).collect())
                            .collect(),
                    )
                })
                .ok_or_else(|| "hilbert(n) needs a size from 1 to 200".to_string()),
            ("vandermonde", 1) => a[0]
                .as_list()
                .map(|x| {
                    matrix::from_rows(
                        x.iter()
                            .map(|v| {
                                (0..x.len())
                                    .map(|k| pow(v.clone(), Expr::int(k as i64)))
                                    .collect()
                            })
                            .collect(),
                    )
                })
                .ok_or_else(|| "vandermonde([x1, x2, ...])".to_string()),
            _ => return None,
        })
    }

    fn calculus(&mut self, name: &str, a: &[Expr]) -> Option<Res> {
        use crate::{diff, integrate, limit, series, sums};
        let digits = numeric::digits();
        let float = |v: Expr, approx: bool| if approx { numeric::n(&v, digits) } else { v };
        Some(match (name, a.len()) {
            ("diff", _) => (|| {
                let mut out = a[0].clone();
                let told = |f: &Expr, var: &str, times: u32, d: &Expr| {
                    crate::trace::step(|| {
                        format!(
                            "the derivative of {f} with respect to {var}{}, by the rules for sums, products, powers and a function of a function: {d}",
                            if times == 1 {
                                String::new()
                            } else {
                                format!(", {times} times")
                            }
                        )
                    });
                };
                if a.len() == 1 {
                    let var = Self::var_of(None, &a[0])?;
                    let d = diff::diff(&out, &var);
                    told(&out, &var, 1, &d);
                    return Ok(simplify(&d));
                }
                let mut i = 1;
                while i < a.len() {
                    let var = Self::var_of(Some(&a[i]), &a[0])?;
                    let times = match a.get(i + 1).and_then(Expr::as_i64) {
                        Some(n) if (0..=1000).contains(&n) => {
                            i += 1;
                            n as u32
                        }
                        Some(_) => {
                            return Err(
                                "diff: the order must be a whole number from 0 to 1000".to_string()
                            );
                        }
                        None => 1,
                    };
                    let d = diff::diff_n(&out, &var, times);
                    told(&out, &var, times, &d);
                    out = d;
                    i += 1;
                }
                Ok(out)
            })(),
            ("derivative'", 1) => Ok(match pick_vars(std::slice::from_ref(&a[0]), 1).pop() {
                Some(v) => simplify(&diff::diff(&a[0], &v)),
                None => Expr::zero(),
            }),
            ("integrate" | "volume", 2 | 3) if region_call(&a[1..]).is_some() => {
                let (vars, region) = region_call(&a[1..]).expect("checked");
                crate::region::over_region(&a[0], &vars, region).map(|(r, approx)| float(r, approx))
            }
            ("integrate" | "volume", n)
                if n >= 3
                    && ranges_of(&a[1..])
                        .is_some_and(|r| r.len() > 1 || matches!(a[1], Expr::List(_))) =>
            {
                let ranges = ranges_of(&a[1..]).expect("checked");
                crate::region::iterated(&a[0], &ranges).map(|(r, approx)| float(r, approx))
            }
            ("nintegrate", n) if n >= 3 && ranges_of(&a[1..]).is_some_and(|r| r.len() > 1) => {
                let ranges = ranges_of(&a[1..]).expect("checked");
                crate::region::numeric_iterated(&a[0], &ranges).map(|r| r.0)
            }
            ("area" | "volume", 1) => crate::region::measure(&a[0]),
            ("perimeter" | "surfacearea", 1) => crate::region::boundary(&a[0]),
            ("centroid", 1) if crate::region::is_region(&a[0]) => crate::region::centroid(&a[0]),
            ("surfacearea", 3) => match region_call(&a[1..]) {
                Some((vars, region)) => crate::region::surface_area(&a[0], &vars, region)
                    .map(|(r, approx)| float(r, approx)),
                None => Err("surfacearea(f, [x, y], region)".to_string()),
            },
            ("area", 2..=5) => (|| {
                let usage = "area(f, x, a, b), area(f, g, x, a, b), area(f, x) or area(f, g, x)";
                let (g, rest) = match a[1].sym_name() {
                    Some(_) => (None, &a[1..]),
                    None => (Some(&a[1]), &a[2..]),
                };
                let (var, ends) = match rest {
                    [v] => (v, None),
                    [v, lo, hi] => (v, Some((lo, hi))),
                    _ => return Err(usage.to_string()),
                };
                let var = var.sym_name().ok_or(usage)?;
                crate::region::area_between(&a[0], g, var, ends).map(|(r, approx)| float(r, approx))
            })(),
            ("arclength", 4) => Self::var_of(Some(&a[1]), &a[0]).and_then(|v| {
                crate::region::arc_length(&a[0], &v, &a[2], &a[3])
                    .map(|(r, approx)| float(r, approx))
            }),
            ("revolutionvolume" | "revolutionarea", 4) => Self::var_of(Some(&a[1]), &a[0])
                .and_then(|v| {
                    let f = &a[0];
                    let integrand = if name == "revolutionvolume" {
                        mul(vec![Expr::pi(), pow(f.clone(), Expr::int(2))])
                    } else {
                        mul(vec![
                            Expr::int(2),
                            Expr::pi(),
                            func("abs", vec![f.clone()]),
                            canon::sqrt(add(vec![
                                Expr::one(),
                                pow(diff::diff(f, &v), Expr::int(2)),
                            ])),
                        ])
                    };
                    let range = crate::region::Range::new(&v, a[2].clone(), a[3].clone());
                    crate::region::iterated(&simplify(&integrand), &[range])
                        .map(|(r, approx)| float(r, approx))
                }),
            ("iterate", 3..=5) => (|| {
                let usage = "iterate(f, x, t), iterate(f, x, t, x0) or iterate(f, x, t, x0, p) with a fixed point p, for example iterate(2x + 1, x, 1/2)";
                let var = a[1].sym_name().ok_or(usage)?;
                // with a fixed point given: regular iteration there, whatever the closed form
                let anchor = a.get(4);
                if anchor.is_some() && !(a[2].is_const() && a[3].is_const()) {
                    return Err(format!(
                        "iterate(f, x, t, x0, p) needs numbers for t and x0: {usage}"
                    ));
                }
                // a whole count of steps at a number: step by step
                if anchor.is_none()
                    && let (Some(n), Some(point)) = (a[2].as_i64(), a.get(3))
                    && point.is_const()
                    && let Some(done) = crate::iterate::whole_steps(&a[0], var, n, point, digits)
                {
                    return done;
                }
                let closed = match anchor {
                    Some(_) => None,
                    None => crate::iterate::symbolic(&a[0], var, &a[2]),
                };
                match (closed, a.get(3)) {
                    (Some(g), None) => Ok(g),
                    (Some(g), Some(point)) => Ok(simplify(&subst(&g, &a[1], point))),
                    (None, Some(point)) if point.is_const() && a[2].is_const() => {
                        let (v, how) =
                            crate::iterate::value(&a[0], var, &a[2], point, anchor, digits)?;
                        let how = format!("iterate: {how}");
                        if !self.notes.contains(&how) {
                            self.notes.push(how);
                        }
                        // noise far below the value is no part of it
                        let size = rug::Float::with_val(64, v.abs_ref());
                        let tiny = |x: &rug::Float| {
                            rug::Float::with_val(64, x.abs_ref()) * 10f64.powi(digits as i32 - 2)
                                < size
                        };
                        let (re, im) = v.into_real_imag();
                        let zero = rug::Float::new(re.prec());
                        let re = if tiny(&re) { zero.clone() } else { re };
                        let im = if tiny(&im) { zero } else { im };
                        Ok(numeric::to_float_expr(
                            rug::Complex::with_val(numeric::bits_for(digits) + 32, (re, im)),
                            digits,
                        ))
                    }
                    (None, _) => {
                        self.notes.push(
                            "no closed form for this iterate: at a number, iterate(f, x, t, x0) or f[t](x0), it is computed by regular iteration at a fixed point".to_string(),
                        );
                        // a readable name for the variable of f[t](x)
                        let mut out = a.to_vec();
                        if var == "#it"
                            && !matches!(&a[0], Expr::Fn(_, inner) if inner.len() == 1 && inner[0].is_sym("#it"))
                        {
                            let name = ["x", "u", "v", "w"]
                                .into_iter()
                                .find(|n| a.iter().all(|e| !e.has_sym(n)))
                                .unwrap_or("x");
                            out[0] = subst(&a[0], &a[1], &Expr::sym(name));
                            out[1] = Expr::sym(name);
                        }
                        Ok(Expr::call("iterate", out))
                    }
                }
            })(),
            ("integrate", 1 | 2) => Self::var_of(a.get(1), &a[0]).map(|v| {
                integrate::integrate(&a[0], &v)
                    .unwrap_or_else(|| Expr::call("integrate", vec![a[0].clone(), Expr::sym(&v)]))
            }),
            ("integrate", 4) => Self::var_of(Some(&a[1]), &a[0]).and_then(|v| {
                if !a[0].free_syms().iter().all(|s| **s == *v)
                    || !a[2].is_const()
                    || !a[3].is_const()
                {
                    // symbolic ends or parameters: the difference of the antiderivative
                    return Ok(integrate::definite_with_parameters(&a[0], &v, &a[2], &a[3])
                        .unwrap_or_else(|| Expr::call("integrate", a.to_vec())));
                }
                integrate::definite(&a[0], &v, &a[2], &a[3]).map(|(r, approx)| float(r, approx))
            }),
            ("nintegrate", 4) => Self::var_of(Some(&a[1]), &a[0]).and_then(|v| {
                integrate::numeric_integral(&a[0], &v, &a[2], &a[3], digits)
                    .map(|z| {
                        numeric::to_float_expr(
                            rug::Complex::with_val(numeric::bits_for(digits) + 32, z),
                            digits,
                        )
                    })
                    .ok_or_else(|| "the numeric integral did not converge".to_string())
            }),
            ("limit", 2..=4) => (|| {
                let (var, mut point, mut dir) = match &a[1] {
                    Expr::Fn(n, r) if &**n == "rule" => (
                        Self::var_of(Some(&r[0]), &a[0])?,
                        r[1].clone(),
                        limit::Dir::Both,
                    ),
                    v => (
                        Self::var_of(Some(v), &a[0])?,
                        a.get(2)
                            .cloned()
                            .ok_or("limit(f, x, a) or limit(f, x -> a)")?,
                        limit::Dir::Both,
                    ),
                };
                if let Expr::Fn(n, r) = &point
                    && &**n == "onesided"
                {
                    dir = if r[1].as_i64() == Some(1) {
                        limit::Dir::Right
                    } else {
                        limit::Dir::Left
                    };
                    point = r[0].clone();
                }
                if let Some(side) = a.last().and_then(Expr::sym_name) {
                    match side {
                        "+" | "right" => dir = limit::Dir::Right,
                        "-" | "left" => dir = limit::Dir::Left,
                        _ => {}
                    }
                }
                limit::limit(&a[0], &var, &point, dir).map(|(r, approx)| float(r, approx))
            })(),
            ("series", 2..=4) => (|| {
                let var = Self::var_of(Some(&a[1]), &a[0])?;
                let point = a.get(2).cloned().unwrap_or_else(Expr::zero);
                let order = a.get(3).and_then(Expr::as_i64).unwrap_or(5);
                if !(0..=200).contains(&order) {
                    return Err(
                        "series: the order must be a whole number from 0 to 200".to_string()
                    );
                }
                crate::trace::step(|| {
                    format!(
                        "the series of {} at {var} = {point} to the order {order}: the known series of its parts are added, multiplied and put into each other, and the terms beyond the order are dropped",
                        a[0]
                    )
                });
                series::series_expr(&a[0], &var, &point, order)
                    .ok_or_else(|| format!("no series found for {} at {var} = {point}", a[0]))
            })(),
            ("sum" | "product", 4) => Self::var_of(Some(&a[1]), &a[0]).and_then(|k| {
                let r = if name == "sum" {
                    sums::sum(&a[0], &k, &a[2], &a[3])
                } else {
                    sums::product(&a[0], &k, &a[2], &a[3])
                };
                r.map(|(v, approx)| float(v, approx))
            }),
            (
                "laplace" | "invlaplace" | "fourier" | "invfourier" | "ztransform"
                | "invztransform",
                3,
            ) => (|| {
                use crate::transform as tr;
                let var = a[1]
                    .sym_name()
                    .ok_or_else(|| format!("{name}(f, x, y): {} is not a variable", a[1]))?;
                let found = match name {
                    "laplace" => tr::laplace(&a[0], var, &a[2]),
                    "invlaplace" => tr::inverse_laplace(&a[0], var, &a[2]),
                    "fourier" => tr::fourier(&a[0], var, &a[2]),
                    "invfourier" => tr::fourier(&a[0], var, &canon::neg(a[2].clone()))
                        .map(|v| simplify(&div(v, mul(vec![Expr::int(2), Expr::pi()])))),
                    "ztransform" => tr::z_transform(&a[0], var, &a[2]),
                    _ => tr::inverse_z(&a[0], var, &a[2]),
                };
                Ok(found.unwrap_or_else(|| {
                    self.notes
                        .push(format!("no closed form found for this {name}"));
                    Expr::call(name, a.to_vec())
                }))
            })(),
            ("fourierseries", 3) => (|| {
                let usage = "fourierseries(f, x, n) or fourierseries(f, [x, a, b], n)";
                let n = a[2].as_i64().filter(|n| (0..=200).contains(n)).ok_or(
                    "fourierseries: the number of harmonics must be a whole number from 0 to 200",
                )?;
                let (var, lo, hi) = match &a[1] {
                    Expr::Sym(v) => (v.to_string(), canon::neg(Expr::pi()), Expr::pi()),
                    Expr::List(items) if items.len() == 3 => (
                        items[0].sym_name().ok_or(usage)?.to_string(),
                        items[1].clone(),
                        items[2].clone(),
                    ),
                    _ => return Err(usage.to_string()),
                };
                crate::transform::fourier_series(&a[0], &var, n, &lo, &hi)
            })(),
            ("residue", 3) => Self::var_of(Some(&a[1]), &a[0])
                .and_then(|v| crate::transform::residue(&a[0], &v, &a[2])),
            ("seriescoeff", 3 | 4) => Self::var_of(Some(&a[1]), &a[0]).and_then(|v| {
                let (point, n) = if a.len() == 4 {
                    (a[2].clone(), &a[3])
                } else {
                    (Expr::zero(), &a[2])
                };
                let n = n
                    .as_i64()
                    .filter(|n| (-1000..=1000).contains(n))
                    .ok_or("seriescoeff: the power must be a whole number")?;
                crate::transform::series_coefficient(&a[0], &v, &point, n)
            }),
            ("pade", 4 | 5) => Self::var_of(Some(&a[1]), &a[0]).and_then(|v| {
                let (point, m, n) = if a.len() == 5 {
                    (a[2].clone(), &a[3], &a[4])
                } else {
                    (Expr::zero(), &a[2], &a[3])
                };
                let degree = |e: &Expr| {
                    e.as_i64()
                        .filter(|k| (0..=60).contains(k))
                        .map(|k| k as usize)
                        .ok_or("pade: the degrees must be whole numbers from 0 to 60")
                };
                crate::transform::pade(&a[0], &v, &point, degree(m)?, degree(n)?)
            }),
            ("grad", 1 | 2) => Self::var_list(a.get(1), &a[0]).map(|vars| {
                Expr::list(
                    vars.iter()
                        .map(|v| simplify(&diff::diff(&a[0], v)))
                        .collect(),
                )
            }),
            ("hessian", 1 | 2) => Self::var_list(a.get(1), &a[0]).map(|vars| {
                Expr::list(
                    vars.iter()
                        .map(|u| {
                            Expr::list(
                                vars.iter()
                                    .map(|v| simplify(&diff::diff(&diff::diff(&a[0], u), v)))
                                    .collect(),
                            )
                        })
                        .collect(),
                )
            }),
            ("laplacian", 1 | 2) => Self::var_list(a.get(1), &a[0]).map(|vars| {
                simplify(&add(vars
                    .iter()
                    .map(|v| diff::diff_n(&a[0], v, 2))
                    .collect()))
            }),
            ("jacobian", 2) => Self::var_list(a.get(1), &a[0]).and_then(|vars| {
                let fs = a[0].as_list().ok_or("jacobian([f, g], [x, y])")?;
                Ok(Expr::list(
                    fs.iter()
                        .map(|f| {
                            Expr::list(vars.iter().map(|v| simplify(&diff::diff(f, v))).collect())
                        })
                        .collect(),
                ))
            }),
            ("divergence", 2) => Self::var_list(a.get(1), &a[0]).and_then(|vars| {
                let fs = a[0]
                    .as_list()
                    .filter(|fs| fs.len() == vars.len())
                    .ok_or("divergence([f, g, h], [x, y, z])")?;
                Ok(simplify(&add(fs
                    .iter()
                    .zip(&vars)
                    .map(|(f, v)| diff::diff(f, v))
                    .collect())))
            }),
            ("curl", 2) => Self::var_list(a.get(1), &a[0]).and_then(|vars| {
                let fs = a[0]
                    .as_list()
                    .filter(|fs| fs.len() == 3 && vars.len() == 3)
                    .ok_or("curl([f, g, h], [x, y, z])")?;
                let d = |i: usize, j: usize| diff::diff(&fs[i], &vars[j]);
                Ok(Expr::list(vec![
                    simplify(&canon::sub(d(2, 1), d(1, 2))),
                    simplify(&canon::sub(d(0, 2), d(2, 0))),
                    simplify(&canon::sub(d(1, 0), d(0, 1))),
                ]))
            }),
            _ => return None,
        })
    }

    /// Algebra, number guessing and linear algebra commands.
    fn commands(&mut self, name: &str, a: &[Expr]) -> Option<Res> {
        use crate::{algebra, ratsimp};
        let mat =
            |e: &Expr, what: &str| matrix::rows(e).ok_or_else(|| format!("{what} needs a matrix"));
        let vector = |e: &Expr, what: &str| {
            e.as_list()
                .map(<[Expr]>::to_vec)
                .ok_or_else(|| format!("{what} needs a vector"))
        };
        let zero_form = |e: &Expr| match e {
            Expr::Rel(Rel::Eq, l, r) => canon::sub((**l).clone(), (**r).clone()),
            other => other.clone(),
        };
        let count = |n: Option<&Expr>| -> Result<usize, String> {
            match n {
                Some(n) => n
                    .as_i64()
                    .filter(|n| (1..=10_000).contains(n))
                    .map(|n| n as usize)
                    .ok_or_else(|| {
                        format!("{name}(x, n): n must be a whole number from 1 to 10000")
                    }),
                None => Ok(20),
            }
        };
        let terms = |a: &[Expr]| -> Result<usize, String> {
            if a.len() == 1 && a[0].as_rat().is_some() {
                Ok(usize::MAX)
            } else {
                count(a.get(1))
            }
        };
        let digits = numeric::digits();
        Some(match (name, a.len()) {
            ("collect", 2) => Self::var_of(Some(&a[1]), &a[0]).map(|v| algebra::collect(&a[0], &v)),
            ("apart", 1 | 2) => Self::var_of(a.get(1), &a[0])
                .map(|v| crate::integrate::apart(&a[0], &v).unwrap_or_else(|| a[0].clone())),
            ("coeff", 2 | 3) => algebra::coeff(&a[0], &a[1], a.get(2))
                .ok_or_else(|| "coeff(p, x, n): p must be a polynomial in x".to_string()),
            ("degree", 1 | 2) => Self::var_of(a.get(1), &a[0]).and_then(|v| {
                algebra::degree(&a[0], &v)
                    .ok_or_else(|| format!("{} is not a polynomial in {v}", a[0]))
            }),
            ("quo" | "rem", 2 | 3) => Self::var_of(a.get(2), &a[0]).and_then(|v| {
                let (q, r) = ratsimp::div_rem(&a[0], &a[1], &v)
                    .ok_or_else(|| format!("{name} needs polynomials in {v}"))?;
                Ok(if name == "quo" { q } else { r })
            }),
            ("gcd" | "lcm", n) if n >= 2 && !a.iter().all(|x| x.as_rat().is_some()) => (|| {
                let mut acc = a[0].clone();
                for b in &a[1..] {
                    let next = if name == "gcd" {
                        ratsimp::poly_gcd(&acc, b)
                    } else {
                        ratsimp::poly_lcm(&acc, b)
                    };
                    acc = next.ok_or_else(|| format!("{name} needs integers or polynomials"))?;
                }
                Ok(acc)
            })(
            ),
            ("resultant", 2 | 3) => Self::var_of(a.get(2), &a[0]).and_then(|v| {
                ratsimp::resultant(&a[0], &a[1], &v)
                    .ok_or_else(|| format!("resultant needs polynomials in {v}"))
            }),
            ("discriminant", 1 | 2) => Self::var_of(a.get(1), &a[0]).and_then(|v| {
                algebra::discriminant(&a[0], &v).ok_or_else(|| {
                    format!("{} is not a polynomial in {v} of degree 1 or more", a[0])
                })
            }),
            ("roots", 1 | 2) => (|| {
                if a[0].as_list().is_some() {
                    return Err("roots([coefficients]) or roots(expression, variable)".to_string());
                }
                let f = zero_form(&a[0]);
                let v = Self::var_of(a.get(1), &f)?;
                let solved = crate::solve::solve(&f, &v)?;
                crate::solve::release(&solved.params);
                Ok(Expr::list(
                    solved
                        .roots
                        .iter()
                        .filter(|r| {
                            !assume::real_mode()
                                || real_solution(&r.value, &solved.params).is_some()
                        })
                        .flat_map(|r| std::iter::repeat_n(r.value.clone(), r.mult as usize))
                        .collect(),
                ))
            })(),
            ("modsolve", 2 | 3) => (|| {
                let usage = "modsolve(x^2 = 1, x, 8) or modsolve(x^2 = 1, 8)";
                let f = zero_form(&a[0]);
                let (var, m) = if a.len() == 3 {
                    (a[1].sym_name().ok_or(usage)?.to_string(), &a[2])
                } else {
                    (Self::var_of(None, &f)?, &a[1])
                };
                let m = m.as_int().ok_or(usage)?;
                let roots = crate::dioph::mod_solve(&f, &var, m)?;
                Ok(Expr::list(roots.into_iter().map(Expr::integer).collect()))
            })(),
            ("diophantine", 1..=3) => (|| {
                let usage =
                    "diophantine(3x + 5y = 1, [x, y]) or diophantine(equation, [x, y], bound)";
                let f = zero_form(&a[0]);
                let vars = match a.get(1) {
                    Some(v @ (Expr::List(_) | Expr::Sym(_))) => Self::var_list(Some(v), &f)?,
                    Some(_) => return Err(usage.to_string()),
                    None => Self::var_list(None, &f)?,
                };
                let bound = match a.get(2) {
                    Some(b) => Some(
                        b.as_i64()
                            .filter(|b| (0..=1_000_000).contains(b))
                            .ok_or("the bound must be a whole number from 0 to 1000000")?,
                    ),
                    None => None,
                };
                let mut notes = Vec::new();
                let r = crate::dioph::diophantine(&f, &vars, bound, &mut notes);
                self.notes.extend(notes);
                r
            })(),
            ("latticereduce", 1) => crate::lattice::reduce(&a[0]),
            ("findrelation", _) => {
                let items: &[Expr] = match a {
                    [Expr::List(items)] => items,
                    other => other,
                };
                crate::lattice::find_relation(items)
            }
            ("minpoly", 1..=3) => (|| {
                let var = match a.get(1) {
                    Some(v) => v
                        .sym_name()
                        .ok_or("minpoly(a, x, n): x must be a variable")?,
                    None => "x",
                };
                let degree = match a.get(2) {
                    Some(n) => n
                        .as_i64()
                        .filter(|n| (1..=40).contains(n))
                        .ok_or("minpoly(a, x, n): n must be a whole number from 1 to 40")?
                        as usize,
                    None => 12,
                };
                self.notes.push(
                    "the polynomial was found with numbers and confirmed with twice the digits"
                        .to_string(),
                );
                crate::lattice::minimal_polynomial(&a[0], var, degree)
            })(),
            ("trigexpand", 1) => Ok(algebra::trigexpand(&a[0])),
            ("trigreduce", 1) => Ok(algebra::trigreduce(&a[0])),
            ("rationalize", 1 | 2) => algebra::rationalize(&a[0], a.get(1)),
            ("identify", 1) => {
                let found = algebra::identify(&a[0]);
                // a long row of decimals that nothing fits stays the fraction that it is
                let long = |r: &crate::num::Rat| r.denom().significant_bits() > 40;
                if found == a[0] && a[0].as_num().is_some_and(|n| long(&n.re) || long(&n.im)) {
                    self.notes
                        .push("no closed form found for this number".to_string());
                }
                Ok(found)
            }
            ("cfrac", 1 | 2) => terms(a)
                .and_then(|n| algebra::cfrac(&a[0], n))
                .map(|terms| Expr::list(terms.into_iter().map(Expr::integer).collect())),
            ("convergents", 1 | 2) => {
                terms(a)
                    .and_then(|n| algebra::cfrac(&a[0], n))
                    .map(|terms| {
                        Expr::list(
                            crate::ntheory::convergents(&terms)
                                .into_iter()
                                .map(Expr::rational)
                                .collect(),
                        )
                    })
            }
            ("interpolate" | "fit", 1..=3) => (|| {
                // the variable is the last argument when it is a symbol
                let (var, rest) = match a.last() {
                    Some(Expr::Sym(_)) if a.len() > 1 => {
                        (a[a.len() - 1].clone(), &a[..a.len() - 1])
                    }
                    _ => (Expr::sym("x"), a),
                };
                if name == "interpolate" {
                    let (xs, ys) = crate::fit::points(rest)?;
                    return crate::fit::interpolate(&xs, &ys, &var);
                }
                let pairs = |e: &Expr| {
                    e.as_list().is_some_and(|d| {
                        !d.is_empty() && d.iter().all(|p| p.as_list().is_some_and(|l| l.len() == 2))
                    })
                };
                let (data, basis): (&[Expr], Vec<Expr>) = match rest {
                    // points and the functions to combine; two plain lists are abscissas and ordinates
                    [data, Expr::List(basis)] if pairs(data) => {
                        (std::slice::from_ref(data), basis.to_vec())
                    }
                    [data, n] if n.as_i64().is_some_and(|n| (0..=60).contains(&n)) => {
                        let n = n.as_i64().expect("checked");
                        (
                            std::slice::from_ref(data),
                            (0..=n).map(|k| pow(var.clone(), Expr::int(k))).collect(),
                        )
                    }
                    data => (data, vec![Expr::one(), var.clone()]),
                };
                let (xs, ys) = crate::fit::points(data)?;
                crate::fit::fit(&xs, &ys, &basis, &var)
            })(),
            ("leastsquares", 2) => (|| {
                let m = mat(&a[0], "leastsquares")?;
                let b = vector(&a[1], "leastsquares")?;
                let t = matrix::transpose(&m);
                let normal = matrix::matmul(&t, &m)?;
                let column: matrix::Rows = b.iter().map(|x| vec![x.clone()]).collect();
                let right: Vec<Expr> = matrix::matmul(&t, &column)?
                    .into_iter()
                    .map(|r| r[0].clone())
                    .collect();
                matrix::linsolve(&normal, &right)
            })(),
            ("criticalpoints", 1 | 2) => Self::var_list(a.get(1), &a[0])
                .and_then(|vars| crate::fit::critical_points(&a[0], &vars)),
            ("nsolve", 2 | 3) if matches!((&a[0], &a[1]), (Expr::List(_), Expr::List(_))) => {
                (|| {
                    let usage =
                        "nsolve([equations], [x, y], [x0, y0]): as many equations as unknowns";
                    let eqs: Vec<Expr> = a[0]
                        .as_list()
                        .ok_or(usage)?
                        .iter()
                        .map(&zero_form)
                        .collect();
                    let vars: Vec<String> = a[1]
                        .as_list()
                        .ok_or(usage)?
                        .iter()
                        .map(|v| v.sym_name().map(str::to_string).ok_or(usage))
                        .collect::<Result<_, _>>()?;
                    if eqs.len() != vars.len() || eqs.is_empty() {
                        return Err(usage.to_string());
                    }
                    let n = vars.len();
                    let starts: Vec<Vec<rug::Complex>> = match a.get(2) {
                        Some(s) => {
                            let items = s.as_list().filter(|l| l.len() == n).ok_or(usage)?;
                            vec![
                                items
                                    .iter()
                                    .map(|x| numeric::evalf(x, 128).map_err(|_| usage.to_string()))
                                    .collect::<Result<_, _>>()?,
                            ]
                        }
                        None => [0.5, 1.0, -1.0, 2.0, -2.0, 0.1, 5.0, -5.0]
                            .iter()
                            .map(|c| {
                                (0..n)
                                    .map(|k| {
                                        rug::Complex::with_val(64, (*c + 0.137 * k as f64, 0.0))
                                    })
                                    .collect()
                            })
                            .chain((0..n).map(|k| {
                                (0..n)
                                    .map(|j| {
                                        rug::Complex::with_val(
                                            64,
                                            (if j == k { 1.3 } else { 0.4 }, 0.6),
                                        )
                                    })
                                    .collect()
                            }))
                            .collect(),
                    };
                    for start in starts {
                        if let Some(found) = crate::fit::newton_system(&eqs, &vars, &start, digits)
                        {
                            let clean = |z: &rug::Complex| {
                                let scale = rug::Float::with_val(64, z.abs_ref()) + 1u32;
                                let tiny = rug::Float::with_val(
                                    64,
                                    &scale >> (numeric::bits_for(digits) - 8),
                                );
                                let part = |x: &rug::Float| {
                                    if rug::Float::with_val(64, x.abs_ref()) < tiny {
                                        rug::Float::new(x.prec())
                                    } else {
                                        x.clone()
                                    }
                                };
                                rug::Complex::with_val(z.prec(), (part(z.real()), part(z.imag())))
                            };
                            let values: Vec<rug::Complex> = found.iter().map(clean).collect();
                            if assume::real_mode() && values.iter().any(|z| !z.imag().is_zero()) {
                                continue;
                            }
                            return Ok(Expr::list(
                                vars.iter()
                                    .zip(values)
                                    .map(|(v, z)| {
                                        Expr::call(
                                            "rule",
                                            vec![Expr::sym(v), numeric::to_float_expr(z, digits)],
                                        )
                                    })
                                    .collect(),
                            ));
                        }
                    }
                    Err("no solution found from these starting points".to_string())
                })()
            }
            ("nsolve", 2 | 3) => (|| {
                let f = zero_form(&a[0]);
                let v = Self::var_of(Some(&a[1]), &f)?;
                let key: std::rc::Rc<str> = std::rc::Rc::from(v.as_str());
                let mut value = |z: &rug::Complex| {
                    let mut env = numeric::Env::new();
                    env.insert(key.clone(), z.clone());
                    numeric::evalf_env(&f, z.prec().0, &env).ok()
                };
                let starts: Vec<rug::Complex> =
                    match a.get(2) {
                        Some(s) => vec![numeric::evalf(s, 128).map_err(
                            |_| "nsolve(equation, x, start): the start must be a number",
                        )?],
                        None => [
                            (0.5, 0.0),
                            (1.0, 0.0),
                            (-1.0, 0.0),
                            (2.0, 0.0),
                            (-2.0, 0.0),
                            (5.0, 0.0),
                            (-5.0, 0.0),
                            (10.0, 0.0),
                            (0.3, 1.0),
                            (0.3, -1.0),
                        ]
                        .iter()
                        .map(|(x, y)| rug::Complex::with_val(64, (*x + 0.013, *y)))
                        .collect(),
                    };
                for s in starts {
                    if let Some(mut z) = crate::numroots::find_root(&mut value, &s, digits) {
                        let scale = rug::Float::with_val(64, z.abs_ref()) + 1u32;
                        if rug::Float::with_val(64, z.imag().abs_ref())
                            < rug::Float::with_val(64, &scale >> (numeric::bits_for(digits) - 8))
                        {
                            z = rug::Complex::with_val(z.prec(), z.real());
                        }
                        if assume::real_mode() && !z.imag().is_zero() {
                            continue;
                        }
                        return Ok(numeric::to_float_expr(z, digits));
                    }
                }
                Err(format!("no root found for {f}"))
            })(),
            ("minimize" | "maximize", 1 | 2 | 4) => Self::var_of(a.get(1), &a[0]).and_then(|v| {
                crate::reals::extremum(
                    &a[0],
                    &v,
                    a.get(2),
                    a.get(3),
                    name == "minimize",
                    &mut self.notes,
                )
            }),
            ("trace", 1) => mat(&a[0], "trace").and_then(|m| matrix::trace(&m)),
            ("rank", 1) => {
                mat(&a[0], "rank").map(|m| Expr::int(matrix::eliminate(m, false).1 as i64))
            }
            ("rref", 1) => {
                mat(&a[0], "rref").map(|m| matrix::from_rows(matrix::eliminate(m, true).0))
            }
            ("nullspace", 1) => mat(&a[0], "nullspace")
                .map(|m| Expr::list(matrix::nullspace(&m).into_iter().map(Expr::list).collect())),
            ("charpoly", 1 | 2) => mat(&a[0], "charpoly").and_then(|m| {
                matrix::charpoly(
                    &m,
                    &a.get(1).cloned().unwrap_or_else(|| Expr::sym("lambda")),
                )
            }),
            ("eigenvalues" | "eigenvectors", 1) => {
                mat(&a[0], name).and_then(|m| Self::eigen(&m, name == "eigenvectors"))
            }
            ("dot", 2) => {
                vector(&a[0], "dot").and_then(|x| matrix::dot_product(&x, &vector(&a[1], "dot")?))
            }
            ("cross", 2) => {
                vector(&a[0], "cross").and_then(|x| matrix::cross(&x, &vector(&a[1], "cross")?))
            }
            ("identity", 1) => a[0]
                .as_i64()
                .filter(|n| (1..=200).contains(n))
                .map(|n| matrix::from_rows(matrix::identity(n as usize)))
                .ok_or_else(|| "identity(n): n must be a whole number from 1 to 200".to_string()),
            ("linsolve", 2) => mat(&a[0], "linsolve")
                .and_then(|m| matrix::linsolve(&m, &vector(&a[1], "linsolve")?)),
            _ => return None,
        })
    }

    /// Eigenvalues with multiplicity, or `[value, [vectors]]` for each distinct one.
    fn eigen(m: &matrix::Rows, vectors: bool) -> Res {
        const VAR: &str = "#ev";
        let p = matrix::charpoly(m, &Expr::sym(VAR))?;
        let solved = crate::solve::solve(&p, VAR)?;
        let digits = numeric::digits();
        let mut out = Vec::new();
        for r in &solved.roots {
            if !vectors {
                out.extend(std::iter::repeat_n(r.value.clone(), r.mult as usize));
                continue;
            }
            let shifted: matrix::Rows = m
                .iter()
                .enumerate()
                .map(|(i, row)| {
                    row.iter()
                        .enumerate()
                        .map(|(j, x)| {
                            if i == j {
                                simplify(&canon::sub(x.clone(), r.value.clone()))
                            } else {
                                x.clone()
                            }
                        })
                        .collect()
                })
                .collect();
            let basis = if r.approx {
                let bits = numeric::bits_for(digits);
                let tiny = rug::Float::with_val(64, 1) >> (bits / 2);
                matrix::nullspace_with(&shifted, &|e| match numeric::evalf(e, bits) {
                    Ok(v) => rug::Float::with_val(64, v.abs_ref()) < tiny,
                    Err(_) => crate::simplify::is_zero(e),
                })
            } else {
                matrix::nullspace(&shifted)
            };
            out.push(Expr::list(vec![
                r.value.clone(),
                Expr::list(basis.into_iter().map(Expr::list).collect()),
            ]));
        }
        Ok(Expr::list(out))
    }

    fn subst(&mut self, a: &[Expr]) -> Res {
        let apply = |e: &Expr, rule: &Expr| -> Res {
            match rule {
                Expr::Rel(Rel::Eq, from, to) => Ok(subst(e, from, to)),
                Expr::Fn(n, r) if &**n == "rule" => Ok(subst(e, &r[0], &r[1])),
                _ => Err("subst(expr, x = value) or subst(expr, x, value)".into()),
            }
        };
        if a.len() == 3 {
            return Ok(subst(&a[0], &a[1], &a[2]));
        }
        match &a[1] {
            Expr::List(rules) => rules.iter().try_fold(a[0].clone(), |e, r| apply(&e, r)),
            rule => apply(&a[0], rule),
        }
    }

    fn assume(&mut self, raw: &[Expr]) -> Res {
        let fact_of = |word: &str| -> Option<Facts> {
            Some(match word.to_ascii_lowercase().as_str() {
                "real" | "reals" => Facts::real(),
                "positive" => Facts::positive(),
                "negative" => Facts::negative(),
                "nonnegative" => Facts::nonnegative(),
                "nonzero" => Facts {
                    nonzero: true,
                    ..Facts::default()
                },
                "integer" | "integers" => Facts::integer(),
                "natural" | "naturals" => Facts {
                    integer: true,
                    real: true,
                    nonnegative: true,
                    ..Facts::default()
                },
                _ => return None,
            })
        };
        let usage = "assume(x > 0), assume(x, real), assume(n, integer)";
        let (name, facts) = match raw {
            [Expr::Sym(v), Expr::Sym(word)] => (v.clone(), fact_of(word).ok_or(usage)?),
            [Expr::Fn(n, a)] if &**n == "element" => match (&a[0], &a[1]) {
                (Expr::Sym(v), Expr::Sym(word)) => (v.clone(), fact_of(word).ok_or(usage)?),
                _ => return Err(usage.into()),
            },
            [Expr::Rel(op, l, r)] => {
                let (v, op, bound) = match (&**l, &**r) {
                    (Expr::Sym(v), bound) => (v.clone(), *op, bound),
                    (bound, Expr::Sym(v)) => (v.clone(), op.flip(), bound),
                    _ => return Err(usage.into()),
                };
                let s = numeric::sign(&self.eval(bound)?).ok_or(usage)?;
                let facts = match (op, s) {
                    (Rel::Gt, s) if s >= 0 => Facts::positive(),
                    (Rel::Ge, s) if s > 0 => Facts::positive(),
                    (Rel::Ge, 0) => Facts::nonnegative(),
                    (Rel::Lt, s) if s <= 0 => Facts::negative(),
                    (Rel::Le, s) if s < 0 => Facts::negative(),
                    (Rel::Ne, 0) => Facts {
                        nonzero: true,
                        ..Facts::default()
                    },
                    _ => Facts::real(),
                };
                (v, facts)
            }
            _ => return Err(usage.into()),
        };
        assume::set(&name, facts);
        Ok(Expr::Bool(true))
    }
}

/// The unknowns of equations: x, y, z, t, u, v, w first, then the other symbols by name.
pub fn pick_vars(eqs: &[Expr], count: usize) -> Vec<String> {
    let mut syms = std::collections::BTreeSet::new();
    for e in eqs {
        e.collect_syms(&mut syms);
    }
    let rank = |s: &str| {
        ["x", "y", "z", "t", "u", "v", "w"]
            .iter()
            .position(|p| *p == s)
            .unwrap_or(99)
    };
    let mut names: Vec<String> = syms.iter().map(|s| s.to_string()).collect();
    names.sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| a.cmp(b)));
    names.truncate(count);
    names
}

/// A function of one argument applies to each entry of a list; min and max take the list itself.
/// Ranges `[x, a, b], [y, c, d], ...` or `x, a, b, y, c, d, ...` of an iterated integral.
fn ranges_of(args: &[Expr]) -> Option<Vec<crate::region::Range>> {
    use crate::region::Range;
    let triple = |v: &[Expr]| Some(Range::new(v[0].sym_name()?, v[1].clone(), v[2].clone()));
    if args.is_empty() {
        return None;
    }
    if args
        .iter()
        .all(|r| matches!(r, Expr::List(v) if v.len() == 3))
    {
        return args
            .iter()
            .map(|r| triple(r.as_list().unwrap_or_default()))
            .collect();
    }
    if args.len().is_multiple_of(3) {
        return args.chunks(3).map(triple).collect();
    }
    None
}

/// `z, region`, `[x, y], region` or `z in region` as the variables and the region.
fn region_call(args: &[Expr]) -> Option<(crate::region::Vars, &Expr)> {
    use crate::region::Vars;
    let (vars, region) = match args {
        [Expr::Fn(n, a)] if &**n == "element" && a.len() == 2 => (&a[0], &a[1]),
        [vars, region] => (vars, region),
        _ => return None,
    };
    if !crate::region::is_region(region) {
        return None;
    }
    let vars = match vars {
        Expr::Sym(s) => Vars::Complex(s.to_string()),
        Expr::List(items) => Vars::Real(
            items
                .iter()
                .map(|i| i.sym_name().map(str::to_string))
                .collect::<Option<_>>()?,
        ),
        _ => return None,
    };
    Some((vars, region))
}

/// A solution as a real value: `None` when it is not real. A family with one whole parameter
/// whose imaginary part vanishes for a single member is cut down to that member.
fn real_solution(value: &Expr, params: &[String]) -> Option<Expr> {
    let digits = numeric::digits();
    let used: Vec<&String> = params.iter().filter(|p| value.has_sym(p)).collect();
    let member = |k: &Expr| {
        used.iter()
            .fold(value.clone(), |v, p| canon::subst_sym(&v, p, k))
    };
    let at = |k: i64| {
        let v = member(&Expr::int(k));
        v.is_const()
            .then(|| numeric::evalf_digits(&v, digits).ok())
            .flatten()
    };
    if used.is_empty() {
        return match at(0) {
            Some(v) if !v.imag().is_zero() => None,
            _ => Some(value.clone()),
        };
    }
    let (Some(v0), Some(v1), Some(v2)) = (at(0), at(1), at(2)) else {
        return Some(value.clone());
    };
    if [&v0, &v1, &v2].iter().all(|v| v.imag().is_zero()) {
        return Some(value.clone());
    }
    if used.len() != 1 {
        return None;
    }
    let slope = rug::Float::with_val(v0.prec().0, v1.imag() - v0.imag());
    if slope.is_zero() {
        return None;
    }
    let k = -(rug::Float::with_val(v0.prec().0, v0.imag() / &slope));
    let whole = k.clone().round();
    if rug::Float::with_val(64, &k - &whole).abs() > 1e-12 {
        return None;
    }
    let candidate = simplify(&member(&Expr::integer(whole.to_integer()?)));
    numeric::evalf_digits(&candidate, digits)
        .is_ok_and(|v| v.imag().is_zero())
        .then_some(candidate)
}

fn listable(name: &str, a: Vec<Expr>) -> Expr {
    match a.as_slice() {
        [Expr::List(items)] if matches!(name, "min" | "max") => func(name, items.to_vec()),
        [Expr::List(_)] if matches!(name, "piecewise" | "and" | "or" | "not") => func(name, a),
        [Expr::List(items)] => Expr::list(
            items
                .iter()
                .map(|x| listable(name, vec![x.clone()]))
                .collect(),
        ),
        _ => func(name, a),
    }
}

/// A list of the solutions of an earlier equation: every entry gives a value to a name and a
/// name has more than one value.
fn alternatives(e: &Expr) -> bool {
    let Expr::List(items) = e else {
        return false;
    };
    let mut names = Vec::new();
    for item in items.iter() {
        match item {
            Expr::Rel(Rel::Eq, l, r) if l.sym_name().is_some_and(|n| !r.has_sym(n)) => {
                names.push(l.sym_name().unwrap_or_default())
            }
            _ => return false,
        }
    }
    let given = names.len();
    names.sort_unstable();
    names.dedup();
    names.len() < given
}

/// A solution as a number of the kind that the unknown is known to be: `None` when it is no
/// such number. A family with an integer parameter gives its real member.
fn fitting(facts: Facts, value: &Expr, params: &[String]) -> Option<Expr> {
    let value = if facts.real {
        real_solution(value, params)?
    } else {
        value.clone()
    };
    if !value.is_const() {
        return Some(value);
    }
    let Ok(v) = numeric::evalf_digits(&value, numeric::digits()) else {
        return Some(value);
    };
    let sign = v.real().cmp0()?;
    let whole = || {
        let off = rug::Float::with_val(64, v.real() - v.real().clone().round());
        off.abs() < 1e-20
    };
    let fits = !(facts.positive && sign != std::cmp::Ordering::Greater)
        && !(facts.negative && sign != std::cmp::Ordering::Less)
        && !(facts.nonnegative && sign == std::cmp::Ordering::Less)
        && !(facts.nonzero && sign == std::cmp::Ordering::Equal && v.imag().is_zero())
        && !(facts.integer && !whole());
    fits.then_some(value)
}

/// The kind of numbers that an unknown is known to be, as in "no positive integer solution".
fn kind(facts: Facts) -> String {
    let mut words = facts.describe();
    if let Some(at) = words.iter().position(|w| *w == "integer") {
        let word = words.remove(at);
        words.push(word);
    }
    words.join(" ")
}

/// The entries of a list of lists, whatever its depth.
fn leaves(e: &Expr, out: &mut Vec<Expr>) {
    match e.as_list() {
        Some(items) => items.iter().for_each(|i| leaves(i, out)),
        None => out.push(e.clone()),
    }
}

/// Why the external program gave no value for a constant that needs it.
fn external_failure(value: &Expr) -> Option<String> {
    if !value.is_const() || !crate::tetration::uses_tet(value) {
        return None;
    }
    match numeric::evalf_digits(value, numeric::digits()) {
        Err(numeric::NumErr::External(failure)) => Some(failure),
        _ => None,
    }
}

/// The name under the first postfix derivative, as in `y'`.
pub(crate) fn primed_name(e: &Expr) -> Option<String> {
    let mut found = None;
    e.walk(&mut |x| {
        if let Expr::Fn(n, a) = x
            && &**n == "derivative'"
        {
            match &a[0] {
                Expr::Sym(s) => {
                    found.get_or_insert_with(|| s.to_string());
                }
                Expr::Fn(f, _) if &**f != "derivative'" => {
                    found.get_or_insert_with(|| f.to_string());
                }
                _ => {}
            }
        }
    });
    found
}

/// Rebuilds an unevaluated tree, replacing the nodes for which `f` has an answer.
pub(crate) fn map_raw(e: &Expr, f: &mut dyn FnMut(&Expr) -> Option<Expr>) -> Expr {
    if let Some(r) = f(e) {
        return r;
    }
    let mut all = |items: &[Expr]| -> std::rc::Rc<[Expr]> {
        items
            .iter()
            .map(|x| map_raw(x, f))
            .collect::<Vec<_>>()
            .into()
    };
    match e {
        Expr::Add(a) => Expr::Add(all(a)),
        Expr::Mul(a) => Expr::Mul(all(a)),
        Expr::List(a) => Expr::List(all(a)),
        Expr::Fn(n, a) => Expr::Fn(n.clone(), all(a)),
        Expr::Pow(b, x) => {
            let pair = all(&[(**b).clone(), (**x).clone()]);
            Expr::Pow(pair[0].clone().into(), pair[1].clone().into())
        }
        Expr::Rel(op, l, r) => {
            let pair = all(&[(**l).clone(), (**r).clone()]);
            Expr::Rel(*op, pair[0].clone().into(), pair[1].clone().into())
        }
        other => other.clone(),
    }
}

/// Convenience for tests and one-shot use: the printed lines of one input.
pub fn run_line(input: &str) -> Result<Vec<String>, String> {
    Session::new().run(input)
}
