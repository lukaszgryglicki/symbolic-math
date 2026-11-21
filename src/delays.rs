//! Delays inside systems of control: a system of running time with dead time is an
//! expression in `s` with `e^(-T*s)` in it. How it answers in time (shifted, and in loops as
//! a sum over the trips around the loop) and in frequency, its margins, the ratios of
//! polynomials that stand for it (Padé), and its sampled form.

use crate::arrays::{tidy, word};
use crate::canon::{add, div, mul, neg, pow, sub, subst};
use crate::control::{Sys, is_system, list_of, poly_add, poly_scaled, ss_to_tf, tf_to_ss, trimmed};
use crate::decomp;
use crate::eval::Session;
use crate::expr::{Expr, Rel};
use crate::matrix::{self, Rows};
use crate::numeric;
use rug::{Complex, Float, Rational};

type Res = Result<Expr, String>;

fn s() -> Expr {
    Expr::sym("s")
}

fn is_exp_of_s(e: &Expr) -> bool {
    matches!(e, Expr::Pow(base, exponent) if base.is_sym("e") && exponent.has_sym("s"))
}

/// An expression in s with a delay in it (no list, no object of a system).
pub(crate) fn is_delayed(e: &Expr) -> bool {
    !matches!(e, Expr::List(_) | Expr::Rel(..)) && !is_system(e) && e.any(&is_exp_of_s)
}

/// `"InputDelay", T` among the arguments, or `InputDelay = T`: the place and the delay.
fn delay_option(a: &[Expr]) -> Option<(usize, usize, Expr)> {
    let named = |e: &Expr| {
        word(e).is_some_and(|w| matches!(w.as_str(), "inputdelay" | "outputdelay" | "iodelay" | "delay"))
    };
    for (k, e) in a.iter().enumerate() {
        if let Expr::Rel(Rel::Eq, left, right) = e
            && named(left)
        {
            return Some((k, 1, (**right).clone()));
        }
        if crate::radix::text_of(e).is_some() && named(e) && k + 1 < a.len() {
            return Some((k, 2, a[k + 1].clone()));
        }
    }
    None
}

/// A system with delays as a ratio of polynomials in s and in E = e^(-base*s): the
/// coefficients of the powers of E, the lowest first.
struct Shape {
    base: Expr,
    num: Vec<Expr>,
    den: Vec<Expr>,
}

impl Session {
    /// Every object of a system inside an expression as its ratio of polynomials.
    fn plain(&mut self, e: &Expr, who: &str) -> Result<Expr, String> {
        if is_system(e) {
            let sys = self.sys_of(e, who)?;
            if sys.ts.is_some() {
                return Err(format!(
                    "{who}: a delay e^(-T*s) and a sampled system do not go together (a sampled delay is a power of 1/z)"
                ));
            }
            return Ok(sys.expr());
        }
        Ok(match e {
            Expr::Add(items) => add(items
                .iter()
                .map(|x| self.plain(x, who))
                .collect::<Result<_, _>>()?),
            Expr::Mul(items) => mul(items
                .iter()
                .map(|x| self.plain(x, who))
                .collect::<Result<_, _>>()?),
            Expr::Pow(base, exponent) => pow(self.plain(base, who)?, self.plain(exponent, who)?),
            other => other.clone(),
        })
    }

    /// A sum or a product in which a delay takes part: an expression in s.
    pub(crate) fn delay_joined(&mut self, sum: bool, items: &[Expr]) -> Res {
        let who = if sum {
            "a sum with a delay"
        } else {
            "a product with a delay"
        };
        let parts: Vec<Expr> = items
            .iter()
            .map(|x| self.plain(x, who))
            .collect::<Result<_, _>>()?;
        let all = if sum { add(parts) } else { mul(parts) };
        self.delay_shown(all, who)
    }

    /// An expression of systems: an object when no delay is left in it.
    fn delay_shown(&mut self, e: Expr, who: &str) -> Res {
        let e = tidy(e);
        match e.any(&is_exp_of_s) {
            true => Ok(e),
            false => Ok(self.sys_of_expr(&e, None, who)?.object()),
        }
    }

    /// The rate c and the rest k of an exponent c*s + k.
    fn rate_of(&mut self, exponent: &Expr, who: &str) -> Result<(Expr, Expr), String> {
        let rate = self.call("simplify", &[crate::diff::diff(exponent, "s")])?;
        let rest = self.call("simplify", &[sub(exponent.clone(), mul(vec![rate.clone(), s()]))])?;
        if rate.has_sym("s") || rest.has_sym("s") {
            return Err(format!(
                "{who}: e^({exponent}) is no delay (a delay is e^(-T*s) with a number T)"
            ));
        }
        Ok((rate, rest))
    }

    /// The delays of an expression measured by one of them, and the expression as a ratio
    /// of polynomials in s and E.
    fn shape(&mut self, e: &Expr, who: &str) -> Result<Shape, String> {
        let mut exponents: Vec<Expr> = Vec::new();
        e.walk(&mut |x| {
            if is_exp_of_s(x)
                && let Expr::Pow(_, exponent) = x
                && !exponents.contains(exponent)
            {
                exponents.push((**exponent).clone());
            }
        });
        let mut rates = Vec::new();
        for exponent in &exponents {
            rates.push(self.rate_of(exponent, who)?);
        }
        let Some(first) = rates.first().map(|r| r.0.clone()) else {
            return Err(format!("{who}: no delay in {e}"));
        };
        // the delays as whole multiples of one (an exponent may be written with the other
        // sign, as e^s/(e^s + 1) for 1/(1 + e^(-s)))
        let mut ratios: Vec<Rational> = Vec::new();
        for (rate, _) in &rates {
            let ratio = self.call("simplify", &[div(rate.clone(), first.clone())])?;
            match ratio.as_rat() {
                Some(r) if !r.is_zero() => ratios.push(r.clone()),
                _ => {
                    return Err(format!(
                        "{who}: the delays of this system have no common measure; pade(system, n) gives a ratio of polynomials that stands for it"
                    ));
                }
            }
        }
        let bottom = ratios.iter().fold(rug::Integer::from(1), |m, r| m.lcm(r.denom()));
        let top = ratios.iter().fold(rug::Integer::new(), |g, r| {
            g.gcd(&rug::Integer::from(r.numer() * &bottom).div_exact(r.denom()))
        });
        let measure = Rational::from((top, bottom));
        let ahead = numeric::real_value(&first, numeric::digits()).is_none_or(|c| c > 0);
        let unit_rate = mul(vec![first.clone(), Expr::rational(measure.clone())]);
        let base = tidy(if ahead { unit_rate } else { neg(unit_rate) });
        if numeric::real_value(&base, numeric::digits()).is_none_or(|t| t <= 0) {
            return Err(format!("{who}: a delay is e^(-T*s) with a number T"));
        }
        let unit = Expr::sym("_E");
        let mut swap: Vec<(Expr, Expr)> = Vec::new();
        for ((exponent, (_, rest)), ratio) in exponents.iter().zip(&rates).zip(&ratios) {
            let power = Rational::from(ratio / &measure);
            let whole = match ahead {
                true => Expr::integer(-power.numer().clone()),
                false => Expr::integer(power.numer().clone()),
            };
            let lift = pow(Expr::e(), rest.clone());
            swap.push((pow(Expr::e(), exponent.clone()), mul(vec![lift, pow(unit.clone(), whole)])));
        }
        let mut replaced = e.clone();
        for (from, to) in &swap {
            replaced = subst(&replaced, from, to);
        }
        if replaced.any(&is_exp_of_s) {
            return Err(format!("{who}: the delays of {e} were not all found"));
        }
        let both = self.call("numden", &[tidy(replaced)])?;
        let [num, den] = &list_of(&both)[..] else {
            return Err(format!("{who}: {e} is no ratio of polynomials in s and its delays"));
        };
        let mut powers = |p: &Expr| -> Result<Vec<Expr>, String> {
            let mut c = self.poly_of(p, &unit, who)?;
            c.reverse();
            Ok(c)
        };
        let (num, den) = (powers(num)?, powers(den)?);
        // (a common power of E in both cancels)
        let common = num
            .iter()
            .position(|c| !c.is_zero())
            .unwrap_or(0)
            .min(den.iter().position(|c| !c.is_zero()).unwrap_or(0));
        Ok(Shape {
            base,
            num: num[common..].to_vec(),
            den: den[common..].to_vec(),
        })
    }

    /// R(s) and T when the system is R(s)*e^(-T*s) with a ratio of polynomials R.
    fn dead_time(&mut self, e: &Expr, who: &str) -> Result<Option<(Sys, Expr)>, String> {
        let shape = self.shape(e, who)?;
        let filled: Vec<usize> = (0..shape.num.len()).filter(|k| !shape.num[*k].is_zero()).collect();
        if shape.den.len() != 1 || filled.len() != 1 {
            return Ok(None);
        }
        let ratio = div(shape.num[filled[0]].clone(), shape.den[0].clone());
        let sys = self.sys_of_expr(&ratio, None, who)?;
        let delay = tidy(mul(vec![shape.base, Expr::int(filled[0] as i64)]));
        Ok(Some((sys, delay)))
    }

    fn dead_time_alone(&mut self, e: &Expr, who: &str) -> Result<(Sys, Expr), String> {
        self.dead_time(e, who)?.ok_or_else(|| {
            format!(
                "{who}: this system has delays inside loops or in parts that are added; {who} takes a ratio of polynomials times one delay e^(-T*s); pade(system, n) gives a ratio of polynomials that stands for it"
            )
        })
    }

    /// The answer in time to an input with the transform `drive`.
    fn delay_response(&mut self, e: &Expr, drive: Expr, times: Option<&Expr>, who: &str) -> Res {
        let shape = self.shape(e, who)?;
        let lead = shape.den[0].clone();
        if lead.is_zero() {
            return Err(format!("{who}: this system would answer before it is asked"));
        }
        let t = Expr::sym("t");
        let prec = crate::mfun::bits();
        let step = crate::mfun::real(&shape.base, prec, who)?;
        let loops = shape.den.len() > 1;
        let last = match (times, loops) {
            (None, false) => shape.num.len() - 1,
            (None, true) => {
                return Err(format!(
                    "{who}: a delay inside a loop goes around without end: {who}(system, t or [t]) gives the values at those times"
                ));
            }
            (Some(at), _) => {
                let mut top = Float::with_val(prec, 0);
                for x in list_of(at) {
                    top = top.max(&crate::mfun::real(&x, prec, who)?);
                }
                let trips = Float::with_val(prec, &top / &step).floor();
                match trips.to_f64() {
                    k if k <= 60.0 || !loops => (k.max(0.0) as usize).min(match loops {
                        true => 60,
                        false => shape.num.len() - 1,
                    }),
                    _ => {
                        return Err(format!(
                            "{who}: the last time is more than 60 delays away; pade(system, n) gives a ratio of polynomials that stands for the system"
                        ));
                    }
                }
            }
        };
        // R_k = (N_k - sum of D_j*R_(k-j))/D_0: what comes out after k trips of the delay
        let mut parts: Vec<Expr> = Vec::new();
        let mut answers: Vec<Option<Expr>> = Vec::new();
        for k in 0..=last {
            let mut top = vec![shape.num.get(k).cloned().unwrap_or_else(Expr::zero)];
            for j in 1..=k.min(shape.den.len() - 1) {
                top.push(neg(mul(vec![shape.den[j].clone(), parts[k - j].clone()])));
            }
            let part = self.call("simplify", &[div(add(top), lead.clone())])?;
            let answer = match part.is_zero() {
                true => None,
                false => {
                    let sys = self.sys_of_expr(&part, None, who)?;
                    Some(self.response(&sys, drive.clone(), None, who)?)
                }
            };
            parts.push(part);
            answers.push(answer);
        }
        match times {
            None => {
                let mut terms = Vec::new();
                for (k, answer) in answers.iter().enumerate() {
                    let Some(answer) = answer else { continue };
                    let shift = tidy(mul(vec![Expr::int(k as i64), shape.base.clone()]));
                    terms.push(match k {
                        0 => answer.clone(),
                        _ => {
                            let later = sub(t.clone(), shift);
                            mul(vec![
                                crate::funcs::func("heaviside", vec![later.clone()]),
                                subst(answer, &t, &later),
                            ])
                        }
                    });
                }
                Ok(add(terms))
            }
            Some(at) => {
                let mut values = Vec::new();
                for x in list_of(at) {
                    let now = crate::mfun::real(&x, prec, who)?;
                    let mut sum = Complex::with_val(prec, 0);
                    for (k, answer) in answers.iter().enumerate() {
                        let Some(answer) = answer else { continue };
                        let shift = Float::with_val(prec, &step * k as u64);
                        if now < shift {
                            break;
                        }
                        let later = crate::mfun::value(&Float::with_val(prec, &now - &shift));
                        let value = subst(answer, &t, &later);
                        sum += numeric::evalf(&value, prec)
                            .map_err(|_| format!("{who}: the answer has no value at t = {x}"))?;
                    }
                    values.push(numeric::to_float_expr(sum, numeric::digits()));
                }
                Ok(match at.as_list() {
                    Some(_) => Expr::list(values),
                    None => values.swap_remove(0),
                })
            }
        }
    }

    /// The value of the system at a place, as a number.
    fn delay_at(e: &Expr, x: &Complex, who: &str) -> Result<Complex, String> {
        let prec = x.prec().0;
        let place = numeric::to_float_expr(x.clone(), numeric::digits() + 12);
        numeric::evalf(&subst(e, &s(), &place), prec)
            .map_err(|_| format!("{who}: the system has no value at s = {place}"))
    }

    /// How many zeros of 1 + R*e^(-T*s) lie in the right half of the plane: those of
    /// 1 + R, and two more or fewer for every crossing of the unit circle that the delay
    /// has turned to -1.
    fn delay_unstable(&mut self, sys: &Sys, delay: &Expr, who: &str) -> Result<i64, String> {
        let prec = crate::mfun::bits();
        let closed = poly_add(&sys.num, &sys.den);
        let mut count = 0i64;
        for root in self.roots_of(&trimmed(closed))? {
            let z = numeric::evalf(&root, prec).map_err(|_| format!("{who}: {root} is no number"))?;
            if !z.real().is_sign_negative() && !z.real().is_zero() {
                count += 1;
            }
        }
        let dead = crate::mfun::real(delay, prec, who)?;
        let pi = Float::with_val(prec, rug::float::Constant::Pi);
        for (w, phase) in self.unit_crossings(sys, who)? {
            // the lag that is left until -180 degrees, from 0 to under a whole turn
            let mut left = Float::with_val(prec, &phase + &pi);
            let turn = Float::with_val(prec, &pi * 2u32);
            while left < 0 {
                left += &turn;
            }
            while left >= turn {
                left -= &turn;
            }
            // the delays T0 + k*turn/w at which a zero is on the axis there
            let first = Float::with_val(prec, &left / &w);
            if dead <= first {
                continue;
            }
            let more = (Float::with_val(prec, &dead - &first) * &w / &turn).ceil();
            let passes = more.to_f64().max(0.0) as i64;
            // the gain falls through 1: the zeros go to the right; it rises: to the left
            let near = Float::with_val(prec, &w * Float::with_val(prec, 1e-9));
            let gain = |x: &Float| -> Result<Float, String> {
                let z = numeric::evalf(&sys.at(&mul(vec![Expr::i(), crate::mfun::value(x)])), prec)
                    .map_err(|_| format!("{who}: no gain at the frequency {x}"))?;
                Ok(Float::with_val(prec, z.abs_ref()))
            };
            let falls = gain(&Float::with_val(prec, &w + &near))? < gain(&Float::with_val(prec, &w - &near))?;
            count += if falls { 2 * passes } else { -2 * passes };
        }
        Ok(count)
    }

    /// The frequencies where the gain of a system is 1, with its phase there (radians).
    fn unit_crossings(&mut self, sys: &Sys, who: &str) -> Result<Vec<(Float, Float)>, String> {
        let prec = crate::mfun::bits();
        let record = self.call("allmargin", &[sys.object()])?;
        let mut field = |name: &str| -> Result<Vec<Expr>, String> {
            Ok(list_of(&self.call("getfield", &[record.clone(), crate::texts::text(name)])?))
        };
        let (margins, places) = (field("PhaseMargin")?, field("PMFrequency")?);
        let pi = Float::with_val(prec, rug::float::Constant::Pi);
        let mut out = Vec::new();
        for (margin, w) in margins.iter().zip(places) {
            let (margin, w) = (crate::mfun::real(margin, prec, who)?, crate::mfun::real(&w, prec, who)?);
            out.push((w, (margin - 180u32) * &pi / 180u32));
        }
        Ok(out)
    }

    /// The margins of the loop R*e^(-T*s).
    fn delay_margins(&mut self, e: &Expr, all: bool, who: &str) -> Res {
        let (sys, delay) = self.dead_time_alone(e, who)?;
        let prec = crate::mfun::bits();
        let dead = crate::mfun::real(&delay, prec, who)?;
        let pi = Float::with_val(prec, rug::float::Constant::Pi);
        let shown = crate::mfun::value;
        // the phase margins: at the frequencies where the gain of R is 1
        let mut phase_margins: Vec<(Float, Float)> = Vec::new();
        for (w, phase) in self.unit_crossings(&sys, who)? {
            let mut margin = (Float::with_val(prec, &phase - Float::with_val(prec, &w * &dead)) + &pi) * 180u32 / &pi;
            while margin > 180 {
                margin -= 360u32;
            }
            while margin <= -180 {
                margin += 360u32;
            }
            phase_margins.push((margin, w));
        }
        // the gain margins: where the response is real and negative, up to ten turns of
        // the delay beyond the corners of R
        let mut corner = Float::with_val(prec, 1);
        for root in self.roots_of(&sys.num)?.into_iter().chain(self.roots_of(&sys.den)?) {
            if let Ok(z) = numeric::evalf(&root, prec) {
                corner = corner.max(&Float::with_val(prec, z.abs_ref()));
            }
        }
        let top = match dead.is_zero() {
            true => Float::with_val(prec, &corner * 100u32),
            false => Float::with_val(prec, &corner * 10u32).max(&(Float::with_val(prec, &pi * 20u32) / &dead)),
        };
        let response = |w: &Float| -> Result<Complex, String> {
            let x = Complex::with_val(prec, (0, w));
            Self::delay_at(e, &x, who)
        };
        let cells = 4000u32;
        let low = Float::with_val(prec, &top / 1_000_000u32);
        let ratio = Float::with_val(prec, &top / &low);
        let place = |k: u32| -> Float {
            use rug::ops::Pow;
            Float::with_val(prec, &low * ratio.clone().pow(Float::with_val(prec, k) / cells))
        };
        let mut gain_margins: Vec<(Float, Float)> = Vec::new();
        let mut before = (place(0), response(&place(0))?);
        for k in 1..=cells {
            let now = (place(k), response(&place(k))?);
            let turned = before.1.imag().is_sign_negative() != now.1.imag().is_sign_negative();
            if turned {
                // (halving until the frequency stands)
                let (mut from, mut to) = (before.0.clone(), now.0.clone());
                let mut at_from = before.1.clone();
                for _ in 0..prec + 8 {
                    let middle = Float::with_val(prec, &from + &to) / 2u32;
                    if middle == from || middle == to {
                        break;
                    }
                    let value = response(&middle)?;
                    if value.imag().is_sign_negative() == at_from.imag().is_sign_negative() {
                        from = middle;
                        at_from = value;
                    } else {
                        to = middle;
                    }
                }
                let value = response(&from)?;
                // (a jump of the phase over a pole on the axis is no crossing)
                let small = Float::with_val(prec, value.abs_ref()) >> 20;
                if value.real().is_sign_negative() && Float::with_val(prec, value.imag().abs_ref()) <= small {
                    gain_margins.push((Float::with_val(prec, value.abs_ref()).recip(), from));
                }
            }
            before = now;
        }
        if all {
            let stable = self.delay_unstable(&sys, &delay, who)? == 0;
            let delays: Vec<Expr> = phase_margins
                .iter()
                .map(|(margin, w)| {
                    let mut lag = Float::with_val(prec, margin * &pi) / 180u32;
                    if lag < 0 {
                        lag += Float::with_val(prec, &pi * 2u32);
                    }
                    shown(&(lag / w))
                })
                .collect();
            return Ok(crate::records::record(vec![
                ("GainMargin", Expr::list(gain_margins.iter().map(|m| shown(&m.0)).collect())),
                ("GMFrequency", Expr::list(gain_margins.iter().map(|m| shown(&m.1)).collect())),
                ("PhaseMargin", Expr::list(phase_margins.iter().map(|m| shown(&m.0)).collect())),
                ("PMFrequency", Expr::list(phase_margins.iter().map(|m| shown(&m.1)).collect())),
                ("DelayMargin", Expr::list(delays)),
                ("DMFrequency", Expr::list(phase_margins.iter().map(|m| shown(&m.1)).collect())),
                ("Stable", Expr::Bool(stable)),
            ]));
        }
        let nearest_gain = gain_margins.iter().min_by(|x, y| {
            let far = |m: &Float| Float::with_val(prec, m.ln_ref()).abs();
            far(&x.0).partial_cmp(&far(&y.0)).unwrap_or(std::cmp::Ordering::Equal)
        });
        let nearest_phase = phase_margins.iter().min_by(|x, y| {
            Float::with_val(prec, x.0.abs_ref())
                .partial_cmp(&Float::with_val(prec, y.0.abs_ref()))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let (gm, wcg) = match nearest_gain {
            Some((m, x)) => (shown(m), shown(x)),
            None => (Expr::inf(), Expr::undefined()),
        };
        let (pm, wcp) = match nearest_phase {
            Some((m, x)) => (shown(m), shown(x)),
            None => (Expr::inf(), Expr::undefined()),
        };
        Ok(Expr::list(vec![gm, pm, wcg, wcp]))
    }

    /// The sampled form of R(s)*e^(-T*s) with the input held between the samples, exact
    /// also for a delay that is no whole number of samples.
    fn delay_sampled(&mut self, sys: &Sys, delay: &Expr, step: &Expr, method: Option<&Expr>, who: &str) -> Res {
        let samples = self.call("simplify", &[div(delay.clone(), step.clone())])?;
        let padded = |sys: Sys, zeros: usize| -> Expr {
            let mut den = sys.den.clone();
            den.extend(vec![Expr::zero(); zeros]);
            Sys { den, ..sys }.object()
        };
        if let Some(whole) = samples.as_i64().filter(|k| *k >= 0) {
            let mut args = vec![sys.object(), step.clone()];
            args.extend(method.cloned());
            let base = self.call("c2d", &args)?;
            let base = self.sys_of(&base, who)?;
            return Ok(padded(base, whole as usize));
        }
        if method.and_then(word).is_some_and(|m| m != "zoh") {
            return Err(format!(
                "{who}: a delay that is no whole number of samples is sampled with the input held between them (\"zoh\"); thiran(T, Ts) stands for the part of a sample otherwise"
            ));
        }
        let prec = crate::mfun::bits();
        let (h, dead) = (crate::mfun::real(step, prec, who)?, crate::mfun::real(delay, prec, who)?);
        if h <= 0 || dead < 0 {
            return Err(format!("{who}: the time between samples is above zero and the delay is not negative"));
        }
        let whole = Float::with_val(prec, &dead / &h).ceil();
        let count = whole.to_f64() as usize;
        // the part of the last sample that the delay takes, above 0 and at most h
        let part = Float::with_val(prec, &dead - Float::with_val(prec, &h * (count - 1) as u64));
        let m = tf_to_ss(&sys.num, &sys.den).map_err(|e| format!("{who}: {e}"))?;
        let size = m[0].first().map_or(0, Vec::len);
        if size == 0 {
            return Ok(padded(Sys { ts: Some(step.clone()), ..sys.clone() }, count));
        }
        // exp([[A, B], [0, 0]]*t) = [[e^(A t), the integral of e^(A x) B over 0..t], [0, 1]]
        let grown = |t: &Float| -> Result<(Rows, Rows), String> {
            let mut joined: Rows = Vec::new();
            for (state, input) in m[0].iter().zip(&m[1]) {
                let mut row: Vec<Expr> = state.iter().map(|x| mul(vec![x.clone(), crate::mfun::value(t)])).collect();
                row.push(mul(vec![input[0].clone(), crate::mfun::value(t)]));
                joined.push(row);
            }
            joined.push(vec![Expr::zero(); size + 1]);
            let numbers = decomp::to_complex(&joined)?;
            let all = matrix::rows(&decomp::from_complex(&crate::matfun::exp(&numbers)))
                .ok_or_else(|| format!("{who}: the exponential of the matrix has no value"))?;
            Ok((
                (0..size).map(|i| all[i][..size].to_vec()).collect(),
                (0..size).map(|i| vec![all[i][size].clone()]).collect(),
            ))
        };
        let (own, _) = grown(&h)?;
        let early = Float::with_val(prec, &h - &part);
        let (shift, first) = grown(&early)?;
        let (_, rest) = grown(&part)?;
        let second = matrix::matmul(&shift, &rest)?;
        let nothing = vec![vec![Expr::zero()]];
        let (num0, den) = ss_to_tf(&[own.clone(), first, m[2].clone(), nothing.clone()])?;
        let (num1, _) = ss_to_tf(&[own, second, m[2].clone(), nothing])?;
        // (z*num0 + num1 + D*den)/(den*z^count)
        let mut lifted = num0;
        lifted.push(Expr::zero());
        let top = poly_add(&poly_add(&lifted, &num1), &poly_scaled(&den, &m[3][0][0]));
        let clean = |p: Vec<Expr>| -> Vec<Expr> {
            p.into_iter().map(|x| numeric::n(&tidy(x), numeric::digits())).collect()
        };
        let mut bottom = clean(den);
        bottom.extend(vec![Expr::zero(); count]);
        Ok(Sys {
            num: trimmed(clean(top)),
            den: bottom,
            ts: Some(step.clone()),
        }
        .object())
    }

    pub(crate) fn delays(&mut self, name: &str, a: &[Expr]) -> Option<Res> {
        self.without_names(|session| session.delays_of(name, a))
    }

    fn delays_of(&mut self, name: &str, a: &[Expr]) -> Option<Res> {
        let n = a.len();
        let first = a.first().is_some_and(is_delayed);
        let pair = matches!(name, "series" | "parallel" | "feedback") && n >= 2 && a[..2].iter().any(is_delayed);
        let option = matches!(name, "tf" | "zpk" | "ss") && delay_option(a).is_some();
        let own = match name {
            "hasdelay" | "totaldelay" | "absorbdelay" => n == 1,
            "thiran" => n == 2,
            "exp" => n == 1 && is_system(&a[0]),
            "pade" => n == 2 && first,
            _ => false,
        };
        if !(first || pair || option || own) {
            return None;
        }
        let who = name;
        Some((|| -> Res {
            let (var, t) = (s(), Expr::sym("t"));
            Ok(match (name, n) {
                ("tf" | "zpk" | "ss", _) if option => {
                    let (at, width, delay) = delay_option(a).ok_or("a delay")?;
                    let mut rest = a.to_vec();
                    rest.drain(at..at + width);
                    if delay_option(&rest).is_some() {
                        return Err(format!("{name}: one delay (the delays of the input and of the output of a system with one input and one output add up)"));
                    }
                    let base = self.call(name, &rest)?;
                    let sys = self.sys_of(&base, who)?;
                    let prec = crate::mfun::bits();
                    let dead = crate::mfun::real(&delay, prec, who)?;
                    if dead < 0 {
                        return Err(format!("{name}: a delay is not negative"));
                    }
                    match &sys.ts {
                        Some(_) => {
                            let whole = delay.as_i64().ok_or_else(|| {
                                format!("{name}: the delay of a sampled system is a whole number of samples")
                            })?;
                            let mut den = sys.den.clone();
                            den.extend(vec![Expr::zero(); whole as usize]);
                            Sys { den, ..sys }.object()
                        }
                        None if dead.is_zero() => base,
                        None => tidy(mul(vec![pow(Expr::e(), neg(mul(vec![delay, var]))), sys.expr()])),
                    }
                }
                ("exp", 1) => {
                    let sys = self.sys_of(&a[0], who)?;
                    let line = sys.den.len() == 1 && sys.num.len() == 2 && sys.num[1].is_zero() && sys.ts.is_none();
                    if !line {
                        return Err("exp of a system: e^(-T*s) is a delay; of another system it is nothing".into());
                    }
                    pow(Expr::e(), tidy(mul(vec![div(sys.num[0].clone(), sys.den[0].clone()), var])))
                }
                ("hasdelay", 1) => Expr::Bool(first),
                ("totaldelay", 1) => match first {
                    true => self.dead_time_alone(&a[0], who)?.1,
                    false => {
                        self.sys_of(&a[0], who)?;
                        Expr::zero()
                    }
                },
                ("absorbdelay", 1) => match first {
                    true => {
                        return Err("absorbdelay: a delay of running time is no ratio of polynomials; pade(system, n) gives one that stands for it".into());
                    }
                    false => self.sys_of(&a[0], who)?.object(),
                },
                ("thiran", 2) => {
                    let prec = crate::mfun::bits();
                    let (dead, step) = (crate::mfun::real(&a[0], prec, who)?, crate::mfun::real(&a[1], prec, who)?);
                    if dead < 0 || step <= 0 {
                        return Err("thiran(T, Ts): the delay T is not negative, the time Ts between samples is above zero".into());
                    }
                    let ratio = self.call("simplify", &[div(a[0].clone(), a[1].clone())])?;
                    let order = Float::with_val(prec, &dead / &step).ceil().to_f64() as i64;
                    if order > 200 {
                        return Err("thiran: 200 samples of delay at most".into());
                    }
                    let mut den = vec![Expr::one()];
                    if ratio.as_i64().is_none() {
                        for k in 1..=order {
                            // a_k = (-1)^k C(N, k) * product of (D - N + i)/(D - N + k + i), i from 0 to N
                            let mut value = Expr::int(if k % 2 == 0 { 1 } else { -1 });
                            let mut choose = Expr::one();
                            for j in 1..=k {
                                choose = mul(vec![choose, Expr::rat(order - k + j, j)]);
                            }
                            value = mul(vec![value, choose]);
                            for i in 0..=order {
                                let above = add(vec![ratio.clone(), Expr::int(i - order)]);
                                let below = add(vec![ratio.clone(), Expr::int(i - order + k)]);
                                value = mul(vec![value, div(above, below)]);
                            }
                            den.push(self.call("simplify", &[value])?);
                        }
                    } else {
                        den.extend(vec![Expr::zero(); order as usize]);
                    }
                    let mut num = den.clone();
                    num.reverse();
                    Sys {
                        num: trimmed(num),
                        den,
                        ts: Some(a[1].clone()),
                    }
                    .object()
                }
                ("pade", 2) => {
                    let order = crate::arrays::whole(&a[1], "the order", 0, 60)?;
                    let mut exponents: Vec<Expr> = Vec::new();
                    a[0].walk(&mut |x| {
                        if is_exp_of_s(x)
                            && let Expr::Pow(_, exponent) = x
                            && !exponents.contains(exponent)
                        {
                            exponents.push((**exponent).clone());
                        }
                    });
                    let mut replaced = a[0].clone();
                    for exponent in exponents {
                        let (rate, rest) = self.rate_of(&exponent, who)?;
                        // (e^(T*s) is 1 over the delay)
                        let ahead = numeric::real_value(&rate, numeric::digits()).is_none_or(|c| c > 0);
                        let dead = tidy(if ahead { rate } else { neg(rate) });
                        if numeric::real_value(&dead, numeric::digits()).is_none_or(|t| t < 0) {
                            return Err("pade: a delay is e^(-T*s) with a number T".into());
                        }
                        let mut both = list_of(&self.call("pade", &[dead, Expr::int(order)])?);
                        if ahead {
                            both.swap(0, 1);
                        }
                        let line = |p: &Expr| -> Expr {
                            let c = list_of(p);
                            let size = c.len();
                            add(c
                                .iter()
                                .enumerate()
                                .map(|(k, x)| mul(vec![x.clone(), pow(var.clone(), Expr::int((size - 1 - k) as i64))]))
                                .collect())
                        };
                        let ratio = mul(vec![pow(Expr::e(), rest), div(line(&both[0]), line(&both[1]))]);
                        replaced = subst(&replaced, &pow(Expr::e(), exponent), &ratio);
                    }
                    self.sys_of_expr(&replaced, None, who)?.object()
                }
                ("series" | "parallel", 2) | ("feedback", 2 | 3) => {
                    let (g, h) = (self.plain(&a[0], who)?, self.plain(&a[1], who)?);
                    let joined = match name {
                        "series" => mul(vec![g, h]),
                        "parallel" => add(vec![g, h]),
                        _ => {
                            let sign = match a.get(2) {
                                Some(x) if *x == Expr::one() => -1,
                                Some(x) if *x == Expr::int(-1) => 1,
                                Some(_) => return Err("feedback(G, H[, sign]): the sign is -1 (the usual, negative feedback) or +1".into()),
                                None => 1,
                            };
                            div(g.clone(), add(vec![Expr::one(), mul(vec![Expr::int(sign), g, h])]))
                        }
                    };
                    self.delay_shown(joined, who)?
                }
                ("tf2sym", 1) => a[0].clone(),
                ("isct", 1) => Expr::Bool(true),
                ("isdt", 1) => Expr::Bool(false),
                ("evalfr", 2) => self.call("simplify", &[subst(&a[0], &var, &a[1])])?,
                ("freqresp", 2) => {
                    let mut values = Vec::new();
                    for w in list_of(&a[1]) {
                        values.push(self.call("simplify", &[subst(&a[0], &var, &mul(vec![Expr::i(), w]))])?);
                    }
                    match a[1].as_list() {
                        Some(_) => Expr::list(values),
                        None => values.swap_remove(0),
                    }
                }
                ("dcgain", 1) => {
                    let shape = self.shape(&a[0], who)?;
                    let still = |p: &[Expr]| tidy(subst(&add(p.to_vec()), &var, &Expr::zero()));
                    let (top, bottom) = (still(&shape.num), still(&shape.den));
                    match bottom.is_zero() {
                        true => Expr::inf(),
                        false => self.call("simplify", &[div(top, bottom)])?,
                    }
                }
                ("pole" | "zero" | "isstable" | "isproper" | "damp", 1) => {
                    let (sys, _) = self.dead_time_alone(&a[0], who)?;
                    self.call(name, &[sys.object()])?
                }
                ("bode" | "nyquist" | "nichols" | "sigma", 1 | 2) => {
                    let prec = crate::mfun::bits();
                    let ws: Vec<Expr> = match a.get(1) {
                        Some(w) => list_of(w),
                        None => match self.dead_time(&a[0], who)? {
                            Some((sys, _)) => list_of(&self.call("bode", &[sys.object()])?)[2].as_list().map(<[Expr]>::to_vec).unwrap_or_default(),
                            None => (0..=60).map(|k| pow(Expr::int(10), Expr::rat(k - 30, 15))).collect(),
                        },
                    };
                    let numbers: Vec<Float> = ws.iter().map(|w| crate::mfun::real(w, prec, who)).collect::<Result<_, _>>()?;
                    let shown = |v: &[Float]| Expr::list(v.iter().map(crate::mfun::value).collect());
                    let mut out = match name {
                        "nyquist" => {
                            let (mut re, mut im) = (Vec::new(), Vec::new());
                            for w in &numbers {
                                let h = Self::delay_at(&a[0], &Complex::with_val(prec, (0, w)), who)?;
                                re.push(h.real().clone());
                                im.push(h.imag().clone());
                            }
                            vec![shown(&re), shown(&im)]
                        }
                        _ => {
                            let (gains, phases) = match self.dead_time(&a[0], who)? {
                                Some((sys, delay)) => {
                                    // the phase of the ratio of polynomials less the lag of the delay
                                    let both = list_of(&self.call("bode", &[sys.object(), Expr::list(ws.clone())])?);
                                    let dead = crate::mfun::real(&delay, prec, who)?;
                                    let pi = Float::with_val(prec, rug::float::Constant::Pi);
                                    let gains: Vec<Float> = list_of(&both[0]).iter().map(|g| crate::mfun::real(g, prec, who)).collect::<Result<_, _>>()?;
                                    let mut phases = Vec::new();
                                    for (p, w) in list_of(&both[1]).iter().zip(&numbers) {
                                        let lag = Float::with_val(prec, w * &dead) * 180u32 / &pi;
                                        phases.push(crate::mfun::real(p, prec, who)? - lag);
                                    }
                                    (gains, phases)
                                }
                                None => {
                                    // without jumps: small steps of the frequency between the ones that are asked
                                    let (mut gains, mut phases) = (Vec::new(), Vec::new());
                                    let mut before: Option<(Float, Float)> = None;
                                    let turn = |h: &Complex| Float::with_val(prec, h.arg_ref()) * 180u32 / Float::with_val(prec, rug::float::Constant::Pi);
                                    for w in &numbers {
                                        let h = Self::delay_at(&a[0], &Complex::with_val(prec, (0, w)), who)?;
                                        let mut phase = turn(&h);
                                        if let Some((old_w, old_phase)) = &before {
                                            let mut last = old_phase.clone();
                                            for k in 1..=256u32 {
                                                let between = Float::with_val(prec, old_w + Float::with_val(prec, w - old_w) * k / 256u32);
                                                let mut here = turn(&Self::delay_at(&a[0], &Complex::with_val(prec, (0, &between)), who)?);
                                                let turns = (Float::with_val(prec, &last - &here) / 360u32).round();
                                                here += turns * 360u32;
                                                last = here;
                                            }
                                            phase = last;
                                        }
                                        before = Some((w.clone(), phase.clone()));
                                        gains.push(Float::with_val(prec, h.abs_ref()));
                                        phases.push(phase);
                                    }
                                    (gains, phases)
                                }
                            };
                            match name {
                                "sigma" => vec![shown(&gains)],
                                _ => vec![shown(&gains), shown(&phases)],
                            }
                        }
                    };
                    if a.get(1).is_none() {
                        out.push(Expr::list(ws));
                    }
                    match (name, out.len()) {
                        ("sigma", 1) => out.swap_remove(0),
                        _ => Expr::list(out),
                    }
                }
                ("margin" | "allmargin", 1) => self.delay_margins(&a[0], name == "allmargin", who)?,
                ("step" | "impulse", 1 | 2) => {
                    let drive = match name {
                        "step" => pow(var.clone(), Expr::int(-1)),
                        _ => Expr::one(),
                    };
                    self.delay_response(&a[0], drive, a.get(1), who)?
                }
                ("lsim", 2 | 3) => {
                    let unknown = |e: &Expr| e.has_fn("laplace") || e.has_fn("invlaplace");
                    let drive = self.call("laplace", &[a[1].clone(), t, var])?;
                    if unknown(&drive) {
                        return Err("lsim of a system with a delay: the transform of this input is not known".into());
                    }
                    self.delay_response(&a[0], drive, a.get(2), who)?
                }
                ("c2d", 2 | 3) => {
                    let (sys, delay) = self.dead_time_alone(&a[0], who)?;
                    self.delay_sampled(&sys, &delay, &a[1], a.get(2), who)?
                }
                _ => return Err(String::new()),
            })
        })())
        .filter(|r| !matches!(r, Err(e) if e.is_empty()))
    }
}

#[cfg(test)]
mod tests {
    fn said(input: &str) -> String {
        crate::eval::run_line(input).map_or_else(|e| format!("error: {e}"), |lines| lines.join("|"))
    }

    #[test]
    fn a_delay_shifts_the_answer() {
        assert_eq!(
            said("step(tf([1], [1, 1], \"InputDelay\", 2))"),
            "heaviside(t - 2)*(1 - e^(2 - t))"
        );
        assert!(said("totaldelay(e^(-2s)/(s + 1)*e^(-s/2))").starts_with("5/2|"));
        assert_eq!(said("hasdelay(tf([1], [1, 1]))"), "false");
        assert_eq!(said("pole(e^(-2s)/(s + 1))"), "[-1]");
        assert_eq!(said("dcgain(3*e^(-2s)/(s + 1))"), "3");
        // two ways with different delays, added
        assert_eq!(
            said("step(e^(-s)/s + e^(-3s)/s, [0, 2, 4])"),
            "[0, 1, 4]"
        );
    }

    #[test]
    fn a_delay_inside_a_loop() {
        // 1/(1 + e^(-s)): 1 - 1 + 1 - ... one delay apart
        assert_eq!(
            said("step(feedback(1, e^(-s)), [0.5, 1.5, 2.5, 3.5])"),
            "[1, 0, 1, 0]"
        );
        assert!(said("step(feedback(1/s, e^(-s)))").contains("without end"));
        assert!(said("evalfr(feedback(1, e^(-s)), 0)").starts_with("1/2|"));
        // y' = 1 - y(t - 1): t, then 1 + 2(t - 1) - (t^2 - 1)/2
        assert_eq!(
            said("step(feedback(1/s, e^(-s)), [1/2, 1, 3/2, 2])"),
            "[0.5, 1, 1.375, 1.5]"
        );
    }

    #[test]
    fn a_delay_in_frequency_and_sampled() {
        assert_eq!(said("pade(e^(-2s), 1)"), "tf([-1, 1], [1, 1])");
        // a whole number of samples: poles at 0
        assert_eq!(
            said("c2d(e^(-2s)/s, 1)"),
            "tf([1], [1, -1, 0, 0], 1)"
        );
        assert_eq!(said("thiran(2, 1)"), "tf([1], [1, 0, 0], 1)");
        assert_eq!(said("thiran(1/2, 1)"), "tf([1/3, 1], [1, 1/3], 1)");
        assert_eq!(
            said("tf([1], [1, -1/2], 1, \"InputDelay\", 2)"),
            "tf([1], [1, -1/2, 0, 0], 1)"
        );
    }
}
