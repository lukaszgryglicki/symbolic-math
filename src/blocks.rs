//! Diagrams of blocks, written as text, and their simulation over time: what Simulink
//! does with drawn diagrams. A diagram is a list of blocks; a block is a list that begins
//! with the name of the signal that it makes and its kind, then come what it needs:
//! numbers, and the names of the signals that it takes. `sim(diagram, time)` gives the
//! record of the time and of every signal. The numbers are doubles, as Simulink's are;
//! running states go by the steps of Runge and Kutta of order four, sampled blocks at
//! their own times, and the times where a source jumps are steps of their own.

use crate::expr::Expr;
use crate::numeric::Env;
use crate::radix::text_of;
use crate::texts::text;
use rug::Complex;
use std::rc::Rc;

type Res = Result<Expr, String>;

#[derive(Clone, Debug)]
enum Kind {
    Constant(f64),
    Step {
        time: f64,
        before: f64,
        after: f64,
    },
    Ramp {
        slope: f64,
        start: f64,
    },
    Sine {
        height: f64,
        rate: f64,
        phase: f64,
        bias: f64,
    },
    Pulse {
        height: f64,
        period: f64,
        width: f64,
        delay: f64,
    },
    Clock,
    Signal {
        times: Vec<f64>,
        values: Vec<f64>,
    },
    Sum(Vec<(f64, usize)>),
    Gain(f64, usize),
    Product(Vec<(bool, usize)>),
    Fcn(Expr, Vec<(Rc<str>, usize)>),
    Plain(fn(f64) -> f64, usize),
    Saturation(f64, f64, usize),
    DeadZone(f64, f64, usize),
    Quantizer(f64, usize),
    Switch {
        first: usize,
        control: usize,
        second: usize,
        threshold: f64,
    },
    Least(Vec<usize>),
    Most(Vec<usize>),
    Lookup {
        places: Vec<f64>,
        values: Vec<f64>,
        input: usize,
    },
    Relay {
        on: f64,
        off: f64,
        high: f64,
        low: f64,
        input: usize,
    },
    RateLimiter {
        up: f64,
        down: f64,
        input: usize,
    },
    Derivative(usize),
    Integrator {
        input: usize,
        start: f64,
    },
    /// Running states: dx = A x + B u, y = C x + D u.
    Linear {
        a: Vec<Vec<f64>>,
        b: Vec<f64>,
        c: Vec<f64>,
        d: f64,
        input: usize,
        start: Vec<f64>,
    },
    Delay {
        time: f64,
        input: usize,
        before: f64,
    },
    UnitDelay {
        every: f64,
        input: usize,
        start: f64,
    },
    Hold {
        every: f64,
        input: usize,
    },
    /// A sampled transfer function: the taps above and below (the first below is 1).
    Sampled {
        above: Vec<f64>,
        below: Vec<f64>,
        every: f64,
        input: usize,
    },
}

const FORMS: &[(&str, &str)] = &[
    ("constant", "[\"y\", \"constant\", value]"),
    (
        "step",
        "[\"y\", \"step\", time, value after (1), value before (0)]",
    ),
    ("ramp", "[\"y\", \"ramp\", slope, start (0)]"),
    (
        "sine",
        "[\"y\", \"sine\", height, radians a second, phase (0), bias (0)]",
    ),
    (
        "pulse",
        "[\"y\", \"pulse\", height, period, width in percent, delay (0)]",
    ),
    ("clock", "[\"y\", \"clock\"]: the time"),
    (
        "signal",
        "[\"y\", \"signal\", [times], [values]]: straight lines between given values",
    ),
    (
        "sum",
        "[\"y\", \"sum\", \"a\", \"-b\", ...]: signals added, with - taken away",
    ),
    ("gain", "[\"y\", \"gain\", k, \"u\"]"),
    (
        "product",
        "[\"y\", \"product\", \"a\", \"/b\", ...]: signals multiplied, with / divided by",
    ),
    (
        "fcn",
        "[\"y\", \"fcn\", expression of the names of signals and t] or [\"y\", \"fcn\", expression of u, \"signal\"]",
    ),
    (
        "abs",
        "[\"y\", \"abs\", \"u\"]; likewise sign, sqrt, exp, log, sin, cos, tan, tanh",
    ),
    (
        "saturation",
        "[\"y\", \"saturation\", lowest, highest, \"u\"]",
    ),
    (
        "deadzone",
        "[\"y\", \"deadzone\", start, end, \"u\"]: zero between them, the distance beyond",
    ),
    ("quantizer", "[\"y\", \"quantizer\", step, \"u\"]"),
    (
        "switch",
        "[\"y\", \"switch\", \"first\", \"control\", \"second\", threshold (0)]: the first while the control is at the threshold or above",
    ),
    ("min", "[\"y\", \"min\", \"a\", \"b\", ...]; likewise max"),
    (
        "lookup",
        "[\"y\", \"lookup\", [places], [values], \"u\"]: a table with straight lines between its entries",
    ),
    (
        "relay",
        "[\"y\", \"relay\", on at, off at, value on, value off, \"u\"]",
    ),
    (
        "ratelimiter",
        "[\"y\", \"ratelimiter\", rising slope, falling slope, \"u\"]",
    ),
    (
        "derivative",
        "[\"y\", \"derivative\", \"u\"]: the slope over the last step",
    ),
    ("integrator", "[\"y\", \"integrator\", \"u\", start (0)]"),
    (
        "tf",
        "[\"y\", \"tf\", [numerator], [denominator], \"u\"]: a transfer function in s",
    ),
    (
        "ss",
        "[\"y\", \"ss\", A, B, C, D, \"u\", [start]]: states with one input and one output",
    ),
    (
        "pid",
        "[\"y\", \"pid\", Kp, Ki, Kd, N (100), \"u\"]: Kp + Ki/s + Kd N/(1 + N/s)",
    ),
    (
        "delay",
        "[\"y\", \"delay\", time, \"u\", value before (0)]: the signal that much later",
    ),
    (
        "unitdelay",
        "[\"y\", \"unitdelay\", sample time, \"u\", start (0)]: the sample before",
    ),
    (
        "zoh",
        "[\"y\", \"zoh\", sample time, \"u\"]: the signal held between samples",
    ),
    (
        "dtf",
        "[\"y\", \"dtf\", [numerator], [denominator], sample time, \"u\"]: a sampled transfer function in z",
    ),
];

struct Diagram {
    names: Vec<String>,
    kinds: Vec<Kind>,
    /// The blocks in an order in which each comes after those that it takes at once, then
    /// those of a loop without a state in it.
    order: Vec<usize>,
    looped: Vec<usize>,
    /// Where the running states of a block begin.
    places: Vec<usize>,
    states: usize,
}

/// What a block remembers from step to step.
#[derive(Clone)]
struct Memory {
    values: Vec<Vec<f64>>,
    past: Vec<Vec<(f64, f64)>>,
}

fn number(e: &Expr, what: &str) -> Result<f64, String> {
    crate::plot::number(e)
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("sim: {what} is a number, {e} is none"))
}

fn numbers(e: &Expr, what: &str) -> Result<Vec<f64>, String> {
    match e.as_list() {
        Some(items) => items.iter().map(|x| number(x, what)).collect(),
        None => Ok(vec![number(e, what)?]),
    }
}

impl Diagram {
    fn read(model: &Expr) -> Result<Diagram, String> {
        let usage = "sim([blocks], time): a block is [\"signal\", \"kind\", ...]; simblocks() lists the kinds";
        let blocks = model.as_list().filter(|b| !b.is_empty()).ok_or(usage)?;
        let mut names = Vec::new();
        for block in blocks {
            let name = block
                .as_list()
                .and_then(|b| b.first())
                .and_then(text_of)
                .ok_or(usage)?;
            if names.iter().any(|n| n == name) {
                return Err(format!("sim: two blocks make the signal {name}"));
            }
            names.push(name.to_string());
        }
        let find = |e: &Expr, block: &str| -> Result<usize, String> {
            let name = text_of(e).ok_or_else(|| {
                format!("sim: the block {block} takes a signal by its name in quotes, {e} is none")
            })?;
            names.iter().position(|n| n == name).ok_or_else(|| {
                format!("sim: the block {block} takes the signal {name}, which no block makes")
            })
        };
        let mut kinds = Vec::new();
        for block in blocks {
            let parts = block.as_list().unwrap_or_default();
            let (name, rest) = (&names[kinds.len()], &parts[1..]);
            let kind = rest
                .first()
                .and_then(text_of)
                .ok_or_else(|| {
                    format!("sim: the block {name} has no kind; simblocks() lists them")
                })?
                .to_lowercase();
            let a = &rest[1..];
            let form = || {
                let known = FORMS.iter().find(|(k, _)| {
                    *k == kind
                        || (matches!(
                            kind.as_str(),
                            "sign" | "sqrt" | "exp" | "log" | "sin" | "cos" | "tan" | "tanh"
                        ) && *k == "abs")
                        || (kind == "max" && *k == "min")
                });
                match known {
                    Some((_, form)) => format!("sim: the block {name}: {form}"),
                    None => format!(
                        "sim: the block {name} is of the kind {kind}, which is not known; simblocks() lists the kinds"
                    ),
                }
            };
            let at = |k: usize, what: &str| -> Result<f64, String> {
                number(a.get(k).ok_or_else(&form)?, what)
            };
            let or = |k: usize, usual: f64, what: &str| -> Result<f64, String> {
                match a.get(k) {
                    Some(e) => number(e, what),
                    None => Ok(usual),
                }
            };
            let input =
                |k: usize| -> Result<usize, String> { find(a.get(k).ok_or_else(&form)?, name) };
            let sample = |v: f64| match v > 0.0 {
                true => Ok(v),
                false => Err(format!(
                    "sim: the sample time of the block {name} is above zero"
                )),
            };
            let most = |n: usize| match a.len() <= n {
                true => Ok(()),
                false => Err(form()),
            };
            let linear = |above: &[f64], below: &[f64], input: usize| -> Result<Kind, String> {
                let below: Vec<f64> = below.iter().copied().skip_while(|c| *c == 0.0).collect();
                let above: Vec<f64> = above.iter().copied().skip_while(|c| *c == 0.0).collect();
                if below.is_empty() || above.len() > below.len() {
                    return Err(format!(
                        "sim: the block {name}: the numerator of a transfer function is of no higher degree than its denominator (the block derivative gives a slope)"
                    ));
                }
                let n = below.len() - 1;
                let lead = below[0];
                let a: Vec<f64> = below[1..].iter().map(|c| c / lead).collect();
                let mut b = vec![0.0; below.len() - above.len()];
                b.extend(above.iter().map(|c| c / lead));
                let d = b[0];
                let c: Vec<f64> = (0..n).map(|k| b[k + 1] - d * a[k]).collect();
                let mut rows = vec![a.iter().map(|x| -x).collect::<Vec<f64>>()];
                for i in 1..n {
                    rows.push((0..n).map(|j| f64::from(u8::from(j + 1 == i))).collect());
                }
                rows.truncate(n);
                Ok(Kind::Linear {
                    a: rows,
                    b: (0..n).map(|i| f64::from(u8::from(i == 0))).collect(),
                    c,
                    d,
                    input,
                    start: vec![0.0; n],
                })
            };
            let signed = |e: &Expr, marks: &[char]| -> Result<(Option<char>, usize), String> {
                let written = text_of(e).ok_or_else(&form)?;
                let mark = written.chars().next().filter(|c| marks.contains(c));
                let plain = written.trim_start_matches(marks);
                let k = names.iter().position(|n| n == plain).ok_or_else(|| {
                    format!("sim: the block {name} takes the signal {plain}, which no block makes")
                })?;
                Ok((mark, k))
            };
            kinds.push(match kind.as_str() {
                "constant" => most(1).and_then(|()| Ok(Kind::Constant(at(0, "the value")?)))?,
                "step" => most(3).and_then(|()| Ok(Kind::Step { time: at(0, "the time of the step")?, after: or(1, 1.0, "the value after")?, before: or(2, 0.0, "the value before")? }))?,
                "ramp" => most(2).and_then(|()| Ok(Kind::Ramp { slope: at(0, "the slope")?, start: or(1, 0.0, "the start")? }))?,
                "sine" => most(4).and_then(|()| Ok(Kind::Sine { height: at(0, "the height")?, rate: at(1, "the frequency")?, phase: or(2, 0.0, "the phase")?, bias: or(3, 0.0, "the bias")? }))?,
                "pulse" => {
                    most(4)?;
                    let (period, width) = (at(1, "the period")?, at(2, "the width")?);
                    if period <= 0.0 || !(0.0..=100.0).contains(&width) {
                        return Err(format!("sim: the block {name}: a period above zero and a width from 0 to 100 percent"));
                    }
                    Kind::Pulse { height: at(0, "the height")?, period, width: width / 100.0, delay: or(3, 0.0, "the delay")? }
                }
                "clock" => most(0).map(|()| Kind::Clock)?,
                "signal" | "fromworkspace" => {
                    most(2)?;
                    let (times, values) = (numbers(a.first().ok_or_else(&form)?, "a time")?, numbers(a.get(1).ok_or_else(&form)?, "a value")?);
                    if times.len() != values.len() || times.is_empty() || times.windows(2).any(|w| w[1] <= w[0]) {
                        return Err(format!("sim: the block {name}: as many values as times, and times that grow"));
                    }
                    Kind::Signal { times, values }
                }
                "sum" | "add" => {
                    let terms: Vec<(f64, usize)> = a.iter().map(|e| signed(e, &['+', '-']).map(|(mark, k)| (if mark == Some('-') { -1.0 } else { 1.0 }, k))).collect::<Result<_, _>>()?;
                    if terms.is_empty() {
                        return Err(form());
                    }
                    Kind::Sum(terms)
                }
                "gain" => most(2).and_then(|()| Ok(Kind::Gain(at(0, "the gain")?, input(1)?)))?,
                "product" => {
                    let factors: Vec<(bool, usize)> = a.iter().map(|e| signed(e, &['*', '/']).map(|(mark, k)| (mark == Some('/'), k))).collect::<Result<_, _>>()?;
                    if factors.is_empty() {
                        return Err(form());
                    }
                    Kind::Product(factors)
                }
                "fcn" | "function" => {
                    most(2)?;
                    let formula = a.first().ok_or_else(&form)?.clone();
                    let mut taken = Vec::new();
                    let alone = match a.get(1) {
                        Some(e) => Some(find(e, name)?),
                        None => None,
                    };
                    let mut strangers = Vec::new();
                    let mut both = Vec::new();
                    formula.walk(&mut |e| {
                        // (e and pi are numbers in a formula: a signal of such a name is told apart)
                        if let Expr::Sym(s) = e
                            && crate::expr::is_const_name(s)
                            && names.iter().any(|n| n.as_str() == &**s)
                        {
                            both.push(s.to_string());
                        }
                        if let Expr::Sym(s) = e
                            && !crate::expr::is_const_name(s)
                            && &**s != "t"
                            && !taken.iter().any(|(n, _): &(Rc<str>, usize)| n == s)
                        {
                            match (names.iter().position(|n| n.as_str() == &**s), alone) {
                                (_, Some(k)) if &**s == "u" => taken.push((s.clone(), k)),
                                (Some(k), _) => taken.push((s.clone(), k)),
                                _ => strangers.push(s.to_string()),
                            }
                        }
                    });
                    if let Some(name_of_both) = both.first() {
                        return Err(format!("sim: the block {name}: {name_of_both} in its formula is a number, and a signal of this diagram has that name too: give the signal another name"));
                    }
                    if let Some(stranger) = strangers.first() {
                        return Err(format!("sim: the block {name}: {stranger} in its formula is no signal (nor t, nor u of the one signal named after the formula)"));
                    }
                    Kind::Fcn(formula, taken)
                }
                "abs" | "sign" | "sqrt" | "exp" | "log" | "sin" | "cos" | "tan" | "tanh" => {
                    most(1)?;
                    let f: fn(f64) -> f64 = match kind.as_str() {
                        "abs" => f64::abs,
                        "sign" => |x| if x > 0.0 { 1.0 } else if x < 0.0 { -1.0 } else { 0.0 },
                        "sqrt" => f64::sqrt,
                        // (the true values rounded, not those of the system: the same on every system)
                        "exp" => crate::detmath::exp,
                        "log" => crate::detmath::ln,
                        "sin" => crate::detmath::sin,
                        "cos" => crate::detmath::cos,
                        "tan" => crate::detmath::tan,
                        _ => crate::detmath::tanh,
                    };
                    Kind::Plain(f, input(0)?)
                }
                "saturation" | "deadzone" => {
                    most(3)?;
                    let (low, high) = (at(0, "the lower end")?, at(1, "the upper end")?);
                    if low > high {
                        return Err(format!("sim: the block {name}: the lower end comes first"));
                    }
                    if kind == "saturation" { Kind::Saturation(low, high, input(2)?) } else { Kind::DeadZone(low, high, input(2)?) }
                }
                "quantizer" => {
                    most(2)?;
                    let step = at(0, "the step")?;
                    if step <= 0.0 {
                        return Err(format!("sim: the block {name}: a step above zero"));
                    }
                    Kind::Quantizer(step, input(1)?)
                }
                "switch" => most(4).and_then(|()| Ok(Kind::Switch { first: input(0)?, control: input(1)?, second: input(2)?, threshold: or(3, 0.0, "the threshold")? }))?,
                "min" | "max" => {
                    let all: Vec<usize> = (0..a.len()).map(&input).collect::<Result<_, _>>()?;
                    if all.is_empty() {
                        return Err(form());
                    }
                    if kind == "min" { Kind::Least(all) } else { Kind::Most(all) }
                }
                "lookup" | "table" => {
                    most(3)?;
                    let (places, values) = (numbers(a.first().ok_or_else(&form)?, "a place")?, numbers(a.get(1).ok_or_else(&form)?, "a value")?);
                    if places.len() != values.len() || places.len() < 2 || places.windows(2).any(|w| w[1] <= w[0]) {
                        return Err(format!("sim: the block {name}: as many values as places (two at least), and places that grow"));
                    }
                    Kind::Lookup { places, values, input: input(2)? }
                }
                "relay" => most(5).and_then(|()| Ok(Kind::Relay { on: at(0, "the place to switch on")?, off: at(1, "the place to switch off")?, high: at(2, "the value on")?, low: at(3, "the value off")?, input: input(4)? }))?,
                "ratelimiter" => most(3).and_then(|()| Ok(Kind::RateLimiter { up: at(0, "the rising slope")?, down: at(1, "the falling slope")?, input: input(2)? }))?,
                "derivative" => most(1).and_then(|()| Ok(Kind::Derivative(input(0)?)))?,
                "integrator" => most(2).and_then(|()| Ok(Kind::Integrator { input: input(0)?, start: or(1, 0.0, "the start")? }))?,
                "tf" => {
                    most(3)?;
                    linear(&numbers(a.first().ok_or_else(&form)?, "a coefficient")?, &numbers(a.get(1).ok_or_else(&form)?, "a coefficient")?, input(2)?)?
                }
                "pid" => {
                    let (p, i, d) = (at(0, "Kp")?, at(1, "Ki")?, at(2, "Kd")?);
                    let (n, signal) = match a.len() {
                        4 => (100.0, input(3)?),
                        5 => (at(3, "the filter of the slope")?, input(4)?),
                        _ => return Err(form()),
                    };
                    // Kp + Ki/s + Kd N s/(s + N) over s (s + N)
                    linear(&[p + d * n, p * n + i, i * n], &[1.0, n, 0.0], signal)?
                }
                "ss" => {
                    if !(5..=6).contains(&a.len()) {
                        return Err(form());
                    }
                    let grid = |e: &Expr, what: &str| -> Result<Vec<Vec<f64>>, String> {
                        match crate::matrix::rows(e) {
                            Some(rows) => rows.iter().map(|r| r.iter().map(|x| number(x, what)).collect()).collect(),
                            None => Ok(vec![numbers(e, what)?]),
                        }
                    };
                    let (a_rows, b, c, d) = (grid(&a[0], "an entry of A")?, grid(&a[1], "an entry of B")?, grid(&a[2], "an entry of C")?, grid(&a[3], "an entry of D")?);
                    let n = a_rows.len();
                    let b: Vec<f64> = b.into_iter().flatten().collect();
                    let c: Vec<f64> = c.into_iter().flatten().collect();
                    let d: Vec<f64> = d.into_iter().flatten().collect();
                    if a_rows.iter().any(|r| r.len() != n) || b.len() != n || c.len() != n || d.len() != 1 {
                        return Err(format!("sim: the block {name}: A is n by n, B has n entries, C has n entries and D one (one input and one output)"));
                    }
                    let start = match a.get(5) {
                        Some(e) => numbers(e, "a start")?,
                        None => vec![0.0; n],
                    };
                    if start.len() != n {
                        return Err(format!("sim: the block {name}: a start for every one of its {n} states"));
                    }
                    Kind::Linear { a: a_rows, b, c, d: d[0], input: input(4)?, start }
                }
                "delay" | "transportdelay" => {
                    most(3)?;
                    let time = at(0, "the delay")?;
                    if time <= 0.0 {
                        return Err(format!("sim: the block {name}: a delay above zero"));
                    }
                    Kind::Delay { time, input: input(1)?, before: or(2, 0.0, "the value before")? }
                }
                "unitdelay" => most(3).and_then(|()| Ok(Kind::UnitDelay { every: sample(at(0, "the sample time")?)?, input: input(1)?, start: or(2, 0.0, "the start")? }))?,
                "zoh" | "hold" => most(2).and_then(|()| Ok(Kind::Hold { every: sample(at(0, "the sample time")?)?, input: input(1)? }))?,
                "dtf" => {
                    most(4)?;
                    let (above, below) = (numbers(a.first().ok_or_else(&form)?, "a coefficient")?, numbers(a.get(1).ok_or_else(&form)?, "a coefficient")?);
                    let below: Vec<f64> = below.into_iter().skip_while(|c| *c == 0.0).collect();
                    let above: Vec<f64> = above.into_iter().skip_while(|c| *c == 0.0).collect();
                    if below.is_empty() || above.len() > below.len() {
                        return Err(format!("sim: the block {name}: the numerator of a sampled transfer function is of no higher degree than its denominator"));
                    }
                    let lead = below[0];
                    let mut taps = vec![0.0; below.len() - above.len()];
                    taps.extend(above.iter().map(|c| c / lead));
                    Kind::Sampled { above: taps, below: below.iter().map(|c| c / lead).collect(), every: sample(at(2, "the sample time")?)?, input: input(3)? }
                }
                _ => return Err(form()),
            });
        }
        // what a block needs at once
        let needs: Vec<Vec<usize>> = kinds
            .iter()
            .map(|kind| match kind {
                Kind::Sum(terms) => terms.iter().map(|(_, k)| *k).collect(),
                Kind::Product(factors) => factors.iter().map(|(_, k)| *k).collect(),
                Kind::Fcn(_, taken) => taken.iter().map(|(_, k)| *k).collect(),
                Kind::Gain(_, k)
                | Kind::Plain(_, k)
                | Kind::Saturation(_, _, k)
                | Kind::DeadZone(_, _, k)
                | Kind::Quantizer(_, k)
                | Kind::RateLimiter { input: k, .. }
                | Kind::Hold { input: k, .. } => vec![*k],
                Kind::Switch {
                    first,
                    control,
                    second,
                    ..
                } => vec![*first, *control, *second],
                Kind::Least(all) | Kind::Most(all) => all.clone(),
                Kind::Lookup { input, .. } => vec![*input],
                Kind::Linear { d, input, .. } if *d != 0.0 => vec![*input],
                Kind::Sampled { above, input, .. } if above[0] != 0.0 => vec![*input],
                _ => Vec::new(),
            })
            .collect();
        let mut order = Vec::new();
        let mut done = vec![false; kinds.len()];
        loop {
            let ready: Vec<usize> = (0..kinds.len())
                .filter(|k| !done[*k] && needs[*k].iter().all(|n| done[*n]))
                .collect();
            if ready.is_empty() {
                break;
            }
            for k in ready {
                done[k] = true;
                order.push(k);
            }
        }
        let looped: Vec<usize> = (0..kinds.len()).filter(|k| !done[*k]).collect();
        let mut places = Vec::new();
        let mut states = 0;
        for kind in &kinds {
            places.push(states);
            states += match kind {
                Kind::Integrator { .. } => 1,
                Kind::Linear { a, .. } => a.len(),
                _ => 0,
            };
        }
        Ok(Diagram {
            names,
            kinds,
            order,
            looped,
            places,
            states,
        })
    }

    /// The value of one block from the values of the others.
    #[allow(clippy::too_many_arguments)]
    fn one(
        &self,
        k: usize,
        t: f64,
        at: f64,
        x: &[f64],
        memory: &Memory,
        hits: &[bool],
        out: &[f64],
    ) -> f64 {
        let place = self.places[k];
        match &self.kinds[k] {
            Kind::Constant(c) => *c,
            Kind::Step {
                time,
                before,
                after,
            } => {
                if at >= *time {
                    *after
                } else {
                    *before
                }
            }
            Kind::Ramp { slope, start } => slope * (t - start).max(0.0),
            Kind::Sine {
                height,
                rate,
                phase,
                bias,
            } => height * crate::detmath::sin(rate * t + phase) + bias,
            Kind::Pulse {
                height,
                period,
                width,
                delay,
            } => {
                let into = (at - delay).rem_euclid(*period);
                if at >= *delay && into < width * period {
                    *height
                } else {
                    0.0
                }
            }
            Kind::Clock => t,
            Kind::Signal { times, values } => between(times, values, t),
            Kind::Sum(terms) => terms.iter().map(|(sign, k)| sign * out[*k]).sum(),
            Kind::Gain(gain, k) => gain * out[*k],
            Kind::Product(factors) => {
                factors.iter().fold(
                    1.0,
                    |all, (under, k)| if *under { all / out[*k] } else { all * out[*k] },
                )
            }
            Kind::Fcn(formula, taken) => {
                let mut env = Env::new();
                env.insert(Rc::from("t"), Complex::with_val(64, (t, 0.0)));
                for (name, k) in taken {
                    env.insert(name.clone(), Complex::with_val(64, (out[*k], 0.0)));
                }
                crate::numeric::evalf_env(formula, 64, &env).map_or(f64::NAN, |z| z.real().to_f64())
            }
            Kind::Plain(f, k) => f(out[*k]),
            Kind::Saturation(low, high, k) => out[*k].clamp(*low, *high),
            Kind::DeadZone(low, high, k) => {
                let u = out[*k];
                if u > *high {
                    u - high
                } else if u < *low {
                    u - low
                } else {
                    0.0
                }
            }
            Kind::Quantizer(step, k) => step * (out[*k] / step).round(),
            Kind::Switch {
                first,
                control,
                second,
                threshold,
            } => {
                if out[*control] >= *threshold {
                    out[*first]
                } else {
                    out[*second]
                }
            }
            Kind::Least(all) => all.iter().map(|k| out[*k]).fold(f64::INFINITY, f64::min),
            Kind::Most(all) => all
                .iter()
                .map(|k| out[*k])
                .fold(f64::NEG_INFINITY, f64::max),
            Kind::Lookup {
                places,
                values,
                input,
            } => between(places, values, out[*input]),
            Kind::Relay { high, low, .. } => {
                if memory.values[k][0] > 0.5 {
                    *high
                } else {
                    *low
                }
            }
            Kind::RateLimiter { up, down, input } => {
                let (last, when) = (memory.values[k][0], memory.values[k][1]);
                match last.is_nan() {
                    true => out[*input],
                    false => out[*input].clamp(last + down * (t - when), last + up * (t - when)),
                }
            }
            Kind::Derivative(_) => memory.values[k][2],
            Kind::Integrator { .. } => x[place],
            Kind::Linear { c, d, input, .. } => {
                c.iter()
                    .enumerate()
                    .map(|(i, c)| c * x[place + i])
                    .sum::<f64>()
                    + if *d != 0.0 { d * out[*input] } else { 0.0 }
            }
            Kind::Delay { time, before, .. } => {
                let past = &memory.past[k];
                let wanted = t - time;
                match past.first() {
                    Some((first, _)) if wanted >= *first => {
                        let next = past.partition_point(|(when, _)| *when <= wanted);
                        match (past.get(next.wrapping_sub(1)), past.get(next)) {
                            (Some((ta, ua)), Some((tb, ub))) if tb > ta => {
                                ua + (ub - ua) * (wanted - ta) / (tb - ta)
                            }
                            (Some((_, ua)), None) => *ua,
                            _ => *before,
                        }
                    }
                    _ => *before,
                }
            }
            Kind::UnitDelay { .. } => memory.values[k][0],
            Kind::Hold { input, .. } => {
                if hits[k] {
                    out[*input]
                } else {
                    memory.values[k][0]
                }
            }
            Kind::Sampled { above, input, .. } => {
                if hits[k] {
                    above[0] * out[*input] + memory.values[k].get(1).copied().unwrap_or(0.0)
                } else {
                    memory.values[k][0]
                }
            }
        }
    }

    /// Every signal at a time. `at` is the time that the jumps of the sources go by: just
    /// inside the step that is being made.
    fn signals(
        &self,
        t: f64,
        at: f64,
        x: &[f64],
        memory: &Memory,
        hits: &[bool],
    ) -> Result<Vec<f64>, String> {
        let mut out = vec![0.0; self.kinds.len()];
        for k in &self.order {
            out[*k] = self.one(*k, t, at, x, memory, hits, &out);
        }
        if self.looped.is_empty() {
            return Ok(out);
        }
        // a loop without a state in it: the values that agree with each other, by Newton
        let n = self.looped.len();
        let gap = |out: &mut Vec<f64>, z: &[f64]| -> Vec<f64> {
            for (i, k) in self.looped.iter().enumerate() {
                out[*k] = z[i];
            }
            self.looped
                .iter()
                .enumerate()
                .map(|(i, k)| self.one(*k, t, at, x, memory, hits, out) - z[i])
                .collect()
        };
        let mut z = vec![0.0; n];
        for round in 0..60 {
            let g = gap(&mut out, &z);
            let size = g.iter().map(|v| v.abs()).fold(0.0, f64::max);
            if size <= 1e-12 * (1.0 + z.iter().map(|v| v.abs()).fold(0.0, f64::max)) {
                for (i, k) in self.looped.iter().enumerate() {
                    out[*k] = z[i];
                }
                return Ok(out);
            }
            if !size.is_finite() || round == 59 {
                break;
            }
            let mut slopes = vec![vec![0.0; n]; n];
            for j in 0..n {
                let step = 1e-7 * (1.0 + z[j].abs());
                let mut moved = z.clone();
                moved[j] += step;
                let other = gap(&mut out, &moved);
                for i in 0..n {
                    slopes[i][j] = (other[i] - g[i]) / step;
                }
            }
            let Some(change) = solved(slopes, g.iter().map(|v| -v).collect()) else {
                break;
            };
            for (v, c) in z.iter_mut().zip(change) {
                *v += c;
            }
        }
        Err(format!(
            "sim: the blocks {} form a loop without a state in it whose values do not settle: put an integrator, a transfer function or a delay into the loop",
            self.looped
                .iter()
                .map(|k| self.names[*k].as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }

    fn rates(
        &self,
        t: f64,
        at: f64,
        x: &[f64],
        memory: &Memory,
        hits: &[bool],
    ) -> Result<Vec<f64>, String> {
        let out = self.signals(t, at, x, memory, hits)?;
        let mut rates = vec![0.0; self.states];
        for (k, kind) in self.kinds.iter().enumerate() {
            let place = self.places[k];
            match kind {
                Kind::Integrator { input, .. } => rates[place] = out[*input],
                Kind::Linear { a, b, input, .. } => {
                    for (i, row) in a.iter().enumerate() {
                        rates[place + i] = row
                            .iter()
                            .enumerate()
                            .map(|(j, c)| c * x[place + j])
                            .sum::<f64>()
                            + b[i] * out[*input];
                    }
                }
                _ => {}
            }
        }
        Ok(rates)
    }
}

/// The value of a table with straight lines between its entries; beyond its ends the
/// first and the last line go on (a signal over time) .
fn between(places: &[f64], values: &[f64], at: f64) -> f64 {
    if places.len() == 1 {
        return values[0];
    }
    let next = places
        .partition_point(|p| *p <= at)
        .clamp(1, places.len() - 1);
    let (a, b) = (next - 1, next);
    values[a] + (values[b] - values[a]) * (at - places[a]) / (places[b] - places[a])
}

/// The answer of a small system of lines, by Gauss with the largest entry first.
#[allow(clippy::needless_range_loop)]
fn solved(mut m: Vec<Vec<f64>>, mut right: Vec<f64>) -> Option<Vec<f64>> {
    let n = right.len();
    for c in 0..n {
        let best = (c..n).max_by(|a, b| {
            m[*a][c]
                .abs()
                .partial_cmp(&m[*b][c].abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;
        if m[best][c].abs() < 1e-14 {
            return None;
        }
        m.swap(c, best);
        right.swap(c, best);
        for r in c + 1..n {
            let factor = m[r][c] / m[c][c];
            for j in c..n {
                m[r][j] -= factor * m[c][j];
            }
            right[r] -= factor * right[c];
        }
    }
    for c in (0..n).rev() {
        right[c] = (right[c] - (c + 1..n).map(|j| m[c][j] * right[j]).sum::<f64>()) / m[c][c];
    }
    Some(right)
}

/// `sim(diagram, time[, "step", h][, "points", n])`.
fn sim(a: &[Expr]) -> Res {
    let usage = "sim([blocks], t or [t0, t1][, \"step\", h][, \"points\", n]): the signals of a diagram of blocks over time, a record of the times t and of every signal; simblocks() lists the kinds of blocks";
    let diagram = Diagram::read(a.first().ok_or(usage)?)?;
    let (start, end) = match a.get(1).ok_or(usage)?.as_list() {
        Some([from, to]) => (
            number(from, "the first time")?,
            number(to, "the last time")?,
        ),
        Some(_) => return Err(usage.into()),
        None => (0.0, number(&a[1], "the time")?),
    };
    if end <= start {
        return Err("sim: the time to stop at comes after the start".into());
    }
    let (mut step, mut points) = ((end - start) / 2000.0, 101usize);
    let choices = &a[2..];
    if !choices.len().is_multiple_of(2) {
        return Err(usage.into());
    }
    for pair in choices.chunks(2) {
        match text_of(&pair[0])
            .unwrap_or_default()
            .to_lowercase()
            .as_str()
        {
            "step" | "maxstep" | "fixedstep" => {
                step = number(&pair[1], "the step")?;
                if step <= 0.0 || (end - start) / step > 2e7 {
                    return Err("sim: a step above zero, and 20 million steps at most".into());
                }
            }
            "points" => {
                points =
                    crate::arrays::whole(&pair[1], "the number of points", 2, 1_000_000)? as usize
            }
            _ => return Err(usage.into()),
        }
    }
    // the times of every sampled block, where sources jump, and where the answer is wanted
    let mut times: Vec<f64> = (0..points)
        .map(|k| start + (end - start) * k as f64 / (points - 1) as f64)
        .collect();
    let mut every: Vec<f64> = Vec::new();
    for kind in &diagram.kinds {
        match kind {
            Kind::Step { time, .. } => times.push(*time),
            Kind::Ramp { start, .. } => times.push(*start),
            Kind::Pulse {
                period,
                width,
                delay,
                ..
            } => {
                let count = ((end - delay) / period).ceil().max(0.0);
                if count > 1e6 {
                    return Err("sim: a pulse with more than a million periods".into());
                }
                for k in 0..count as usize {
                    times.extend([
                        delay + k as f64 * period,
                        delay + (k as f64 + width) * period,
                    ]);
                }
            }
            Kind::Signal { times: knots, .. } => times.extend(knots),
            Kind::Delay { time, .. } => step = step.min(*time),
            Kind::UnitDelay { every: ts, .. }
            | Kind::Hold { every: ts, .. }
            | Kind::Sampled { every: ts, .. } => every.push(*ts),
            _ => {}
        }
    }
    for ts in &every {
        if (end - start) / ts > 2e6 {
            return Err("sim: a sampled block with more than two million samples".into());
        }
        let first = (start / ts).ceil() as i64;
        let mut k = first;
        while k as f64 * ts <= end * (1.0 + 1e-12) {
            times.push(k as f64 * ts);
            k += 1;
        }
    }
    let near = 1e-9 * (end - start);
    times.retain(|t| *t >= start - near && *t <= end + near);
    times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    times.dedup_by(|b, a| (*b - *a).abs() <= near);
    let wanted: Vec<f64> = (0..points)
        .map(|k| start + (end - start) * k as f64 / (points - 1) as f64)
        .collect();
    // the states and what the blocks remember at the start
    let mut x = vec![0.0; diagram.states];
    let mut memory = Memory {
        values: vec![Vec::new(); diagram.kinds.len()],
        past: vec![Vec::new(); diagram.kinds.len()],
    };
    for (k, kind) in diagram.kinds.iter().enumerate() {
        let place = diagram.places[k];
        match kind {
            Kind::Integrator { start, .. } => x[place] = *start,
            Kind::Linear { start, .. } => x[place..place + start.len()].copy_from_slice(start),
            Kind::Relay { .. } => memory.values[k] = vec![0.0],
            Kind::RateLimiter { .. } => memory.values[k] = vec![f64::NAN, start],
            Kind::Derivative(_) => memory.values[k] = vec![f64::NAN, 0.0, 0.0],
            Kind::UnitDelay { start, .. } => memory.values[k] = vec![*start, *start],
            Kind::Hold { .. } => memory.values[k] = vec![0.0],
            Kind::Sampled { below, .. } => memory.values[k] = vec![0.0; below.len()],
            _ => {}
        }
    }
    let none = vec![false; diagram.kinds.len()];
    let on_grid = |t: f64, ts: f64| ((t / ts).round() * ts - t).abs() <= near;
    // what the memories that follow the signals keep of a time
    let follow = |t: f64, out: &[f64], memory: &mut Memory| {
        for (k, kind) in diagram.kinds.iter().enumerate() {
            match kind {
                Kind::Relay { on, off, input, .. } => {
                    if out[*input] >= *on {
                        memory.values[k][0] = 1.0;
                    } else if out[*input] <= *off {
                        memory.values[k][0] = 0.0;
                    }
                }
                Kind::RateLimiter { .. } => memory.values[k] = vec![out[k], t],
                Kind::Derivative(input) => {
                    let (last, when) = (memory.values[k][0], memory.values[k][1]);
                    let slope = if last.is_nan() || t <= when {
                        memory.values[k][2]
                    } else {
                        (out[*input] - last) / (t - when)
                    };
                    memory.values[k] = vec![out[*input], t, slope];
                }
                Kind::Delay { input, .. } if memory.past[k].last() != Some(&(t, out[*input])) => {
                    memory.past[k].push((t, out[*input]));
                }
                _ => {}
            }
        }
    };
    // what happens at a time of the list: the memories take what the signals were just
    // before it, the sampled blocks take their samples, and the signals are those just
    // after it (a source that jumps there has jumped)
    let arrive = |t: f64,
                  x: &[f64],
                  memory: &mut Memory,
                  first: bool|
     -> Result<Vec<f64>, String> {
        if !first {
            let before = diagram.signals(t, t - near, x, memory, &none)?;
            follow(t, &before, memory);
        }
        let hits: Vec<bool> = diagram
            .kinds
            .iter()
            .map(|kind| match kind {
                Kind::UnitDelay { every, .. }
                | Kind::Hold { every, .. }
                | Kind::Sampled { every, .. } => on_grid(t, *every),
                _ => false,
            })
            .collect();
        for (k, kind) in diagram.kinds.iter().enumerate() {
            if hits[k] && matches!(kind, Kind::UnitDelay { .. }) {
                memory.values[k][0] = memory.values[k][1];
            }
        }
        let out = diagram.signals(t, t + near, x, memory, &hits)?;
        for (k, kind) in diagram.kinds.iter().enumerate() {
            match kind {
                Kind::UnitDelay { input, .. } if hits[k] => memory.values[k][1] = out[*input],
                Kind::Hold { .. } if hits[k] => memory.values[k][0] = out[k],
                Kind::Sampled {
                    above,
                    below,
                    input,
                    ..
                } if hits[k] => {
                    let (u, y) = (out[*input], out[k]);
                    let order = below.len() - 1;
                    let old = memory.values[k].clone();
                    for i in 1..=order {
                        memory.values[k][i] =
                            above[i] * u - below[i] * y + old.get(i + 1).copied().unwrap_or(0.0);
                    }
                    memory.values[k][0] = y;
                }
                // (after a jump: a relay switches at once, a delay keeps both values; the
                // limiter of slopes goes on from where it was)
                Kind::Relay { on, off, input, .. } => {
                    if out[*input] >= *on {
                        memory.values[k][0] = 1.0;
                    } else if out[*input] <= *off {
                        memory.values[k][0] = 0.0;
                    }
                }
                Kind::Delay { input, .. } if memory.past[k].last() != Some(&(t, out[*input])) => {
                    memory.past[k].push((t, out[*input]));
                }
                Kind::RateLimiter { .. } if first => memory.values[k] = vec![out[k], t],
                Kind::Derivative(input) if first => memory.values[k] = vec![out[*input], t, 0.0],
                _ => {}
            }
        }
        diagram.signals(t, t + near, x, memory, &none)
    };
    let mut rows: Vec<Vec<f64>> = Vec::with_capacity(points);
    let mut shown = arrive(times[0], &x, &mut memory, true)?;
    let mut next_wanted = 0;
    if (times[0] - wanted[0]).abs() <= near {
        rows.push(shown.clone());
        next_wanted = 1;
    }
    for pair in times.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        let parts = ((to - from) / step).ceil().max(1.0) as usize;
        let h = (to - from) / parts as f64;
        let edge = near.min((to - from) / 4.0);
        let inside = |t: f64| t.clamp(from + edge, to - edge);
        for part in 0..parts {
            let t = from + h * part as f64;
            if diagram.states > 0 {
                let k1 = diagram.rates(t, inside(t), &x, &memory, &none)?;
                let moved = |k: &[f64], by: f64| -> Vec<f64> {
                    x.iter().zip(k).map(|(x, k)| x + by * k).collect()
                };
                let k2 = diagram.rates(
                    t + h / 2.0,
                    inside(t + h / 2.0),
                    &moved(&k1, h / 2.0),
                    &memory,
                    &none,
                )?;
                let k3 = diagram.rates(
                    t + h / 2.0,
                    inside(t + h / 2.0),
                    &moved(&k2, h / 2.0),
                    &memory,
                    &none,
                )?;
                let k4 = diagram.rates(t + h, inside(t + h), &moved(&k3, h), &memory, &none)?;
                for i in 0..x.len() {
                    x[i] += h / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
                }
            }
            if x.iter().any(|v| !v.is_finite()) {
                return Err(format!(
                    "sim: the states grow beyond every number near t = {}",
                    crate::plot::label(t + h)
                ));
            }
            if part + 1 < parts {
                // (inside a step: the memories go on, the samples wait)
                let out = diagram.signals(t + h, inside(t + h), &x, &memory, &none)?;
                follow(t + h, &out, &mut memory);
            }
        }
        shown = arrive(to, &x, &mut memory, false)?;
        if next_wanted < points && (to - wanted[next_wanted]).abs() <= near {
            rows.push(shown.clone());
            next_wanted += 1;
        }
    }
    let number_of = crate::matfile::double;
    let mut fields = vec![
        text("t"),
        Expr::list(
            wanted
                .iter()
                .take(rows.len())
                .map(|t| number_of(*t))
                .collect(),
        ),
    ];
    for (k, name) in diagram.names.iter().enumerate() {
        fields.push(text(name));
        fields.push(Expr::list(
            rows.iter().map(|row| number_of(row[k])).collect(),
        ));
    }
    Ok(Expr::call("struct", fields))
}

pub(crate) fn call(name: &str, a: &[Expr]) -> Option<Res> {
    Some(match (name, a.len()) {
        ("sim", 2..) => sim(a),
        ("simblocks", 0) => Ok(Expr::list(
            FORMS.iter().map(|(_, form)| text(form)).collect(),
        )),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    fn said(input: &str) -> String {
        crate::eval::run_line(input).map_or_else(|e| format!("error: {e}"), |lines| lines.join("|"))
    }

    fn last(input: &str) -> f64 {
        said(input)
            .rsplit('|')
            .next()
            .and_then(|v| v.parse().ok())
            .unwrap_or(f64::NAN)
    }

    #[test]
    fn loops_of_blocks_follow_their_formulas() {
        // 1/(s + 1) in a loop with the gain 2: y = 2/3 (1 - e^(-3 t))
        let y = last(
            "r := sim([[\"u\", \"step\", 0], [\"e\", \"sum\", \"u\", \"-y\"], [\"v\", \"gain\", 2, \"e\"], [\"y\", \"tf\", [1], [1, 1], \"v\"]], 2); part(getfield(r, \"y\"), 101)",
        );
        assert!(
            (y - 2.0 / 3.0 * (1.0 - (-6.0f64).exp())).abs() < 1e-11,
            "{y}"
        );
        // an oscillator of two integrators: cos(t)
        let y = last(
            "r := sim([[\"x\", \"integrator\", \"v\", 1], [\"v\", \"integrator\", \"a\"], [\"a\", \"gain\", -1, \"x\"]], 3); part(getfield(r, \"x\"), 101)",
        );
        assert!((y - 3f64.cos()).abs() < 1e-11, "{y}");
        // a formula: Van der Pol's equation keeps its size
        let y = last(
            "r := sim([[\"x\", \"integrator\", \"v\", 2], [\"v\", \"integrator\", \"a\"], [\"a\", \"fcn\", (1 - x^2)*v - x]], 20, \"points\", 401); max(getfield(r, \"x\"))",
        );
        assert!((y - 2.0086).abs() < 2e-3, "{y}");
        // sampled: the sum of the samples of a step, one sample late
        assert_eq!(
            said(
                "getfield(sim([[\"u\", \"constant\", 1], [\"s\", \"sum\", \"u\", \"d\"], [\"d\", \"unitdelay\", 1, \"s\"]], 5, \"points\", 6), \"d\")"
            ),
            "[0, 1, 2, 3, 4, 5]"
        );
        assert_eq!(
            said(
                "getfield(sim([[\"u\", \"clock\"], [\"y\", \"dtf\", [1, 0], [1, -0.5], 1, \"u\"]], 4, \"points\", 5), \"y\")"
            ),
            "[0, 1, 2.5, 4.25, 6.125]"
        );
        // a delay shifts, a loop without a state settles
        let y = last(
            "r := sim([[\"u\", \"sine\", 1, 2], [\"y\", \"delay\", 0.5, \"u\"]], 3); part(getfield(r, \"y\"), 101)",
        );
        assert!((y - 5f64.sin()).abs() < 1e-6, "{y}");
        assert_eq!(
            said(
                "getfield(sim([[\"u\", \"constant\", 6], [\"y\", \"sum\", \"u\", \"-h\"], [\"h\", \"gain\", 2, \"y\"]], 1, \"points\", 2), \"y\")"
            ),
            "[2, 2]"
        );
        assert!(said("sim([[\"y\", \"gain\", 2, \"z\"]], 1)").contains("which no block makes"));
        assert!(said("sim([[\"y\", \"wobble\", 2]], 1)").contains("simblocks()"));
        assert!(
            said("sim([[\"c\", \"constant\", 1], [\"y\", \"sum\", \"y\", \"c\"]], 1)")
                .contains("loop without a state")
        );
    }
}
