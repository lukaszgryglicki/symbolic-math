//! Probability distributions: densities, distribution functions, quantiles and moments from a
//! table of formulas.

use crate::canon::{add, div, mul, sub};
use crate::eval::Session;
use crate::expr::{Expr, Rel};
use crate::numeric;
use crate::simplify::simplify;
use rug::Float;
use std::cell::RefCell;
use std::collections::HashMap;

type Res = Result<Expr, String>;

struct Dist {
    name: &'static str,
    /// Parameter names with their defaults; a parameter without one must be given.
    params: &'static [(&'static str, &'static str)],
    discrete: bool,
    /// The ends of the support.
    support: (&'static str, &'static str),
    /// Where the density is not zero, as a condition on `x`; empty for everywhere.
    inside: &'static str,
    /// What the parameters must satisfy; empty for anything.
    valid: &'static str,
    /// Density (or probability of the value `x`) inside the support.
    pdf: &'static str,
    cdf: &'static str,
    quantile: &'static str,
    mean: &'static str,
    variance: &'static str,
    median: &'static str,
    mode: &'static str,
    skewness: &'static str,
    kurtosis: &'static str,
    mgf: &'static str,
}

const NONE: Dist = Dist {
    name: "",
    params: &[],
    discrete: false,
    support: ("-inf", "inf"),
    inside: "",
    valid: "",
    pdf: "",
    cdf: "",
    quantile: "",
    mean: "",
    variance: "",
    median: "",
    mode: "",
    skewness: "",
    kurtosis: "",
    mgf: "",
};

const DISTRIBUTIONS: &[Dist] = &[
    Dist {
        name: "normaldist",
        valid: "s > 0",
        params: &[("m", "0"), ("s", "1")],
        pdf: "e^(-(x - m)^2/(2s^2))/(s*sqrt(2pi))",
        cdf: "(1 + erf((x - m)/(s*sqrt(2))))/2",
        quantile: "m + s*sqrt(2)*erfinv(2q - 1)",
        mean: "m",
        variance: "s^2",
        median: "m",
        mode: "m",
        skewness: "0",
        kurtosis: "3",
        mgf: "e^(m*t + s^2*t^2/2)",
        ..NONE
    },
    Dist {
        name: "uniformdist",
        valid: "a < b",
        params: &[("a", "0"), ("b", "1")],
        support: ("a", "b"),
        inside: "a <= x and x <= b",
        pdf: "1/(b - a)",
        cdf: "piecewise(x < a, 0, x <= b, (x - a)/(b - a), 1)",
        quantile: "a + q*(b - a)",
        mean: "(a + b)/2",
        variance: "(b - a)^2/12",
        median: "(a + b)/2",
        skewness: "0",
        kurtosis: "9/5",
        mgf: "(e^(b*t) - e^(a*t))/(t*(b - a))",
        ..NONE
    },
    Dist {
        name: "exponentialdist",
        valid: "r > 0",
        params: &[("r", "1")],
        support: ("0", "inf"),
        inside: "x >= 0",
        pdf: "r*e^(-r*x)",
        cdf: "piecewise(x >= 0, 1 - e^(-r*x), 0)",
        quantile: "-ln(1 - q)/r",
        mean: "1/r",
        variance: "1/r^2",
        median: "ln(2)/r",
        mode: "0",
        skewness: "2",
        kurtosis: "9",
        mgf: "r/(r - t)",
        ..NONE
    },
    Dist {
        name: "gammadist",
        valid: "k > 0 and h > 0",
        params: &[("k", ""), ("h", "1")],
        support: ("0", "inf"),
        inside: "x > 0",
        pdf: "x^(k - 1)*e^(-x/h)/(gamma(k)*h^k)",
        cdf: "piecewise(x > 0, gammap(k, x/h), 0)",
        mean: "k*h",
        variance: "k*h^2",
        mode: "piecewise(k >= 1, (k - 1)*h, 0)",
        skewness: "2/sqrt(k)",
        kurtosis: "3 + 6/k",
        mgf: "(1 - h*t)^(-k)",
        ..NONE
    },
    Dist {
        name: "chisquaredist",
        valid: "k > 0",
        params: &[("k", "")],
        support: ("0", "inf"),
        inside: "x > 0",
        pdf: "x^(k/2 - 1)*e^(-x/2)/(2^(k/2)*gamma(k/2))",
        cdf: "piecewise(x > 0, gammap(k/2, x/2), 0)",
        mean: "k",
        variance: "2k",
        mode: "max(k - 2, 0)",
        skewness: "sqrt(8/k)",
        kurtosis: "3 + 12/k",
        mgf: "(1 - 2t)^(-k/2)",
        ..NONE
    },
    Dist {
        name: "studenttdist",
        valid: "v > 0",
        params: &[("v", "")],
        pdf: "gamma((v + 1)/2)/(sqrt(v*pi)*gamma(v/2))*(1 + x^2/v)^(-(v + 1)/2)",
        cdf: "1/2 + x*gamma((v + 1)/2)/(sqrt(v*pi)*gamma(v/2))*hyp2f1(1/2, (v + 1)/2, 3/2, -x^2/v)",
        mean: "piecewise(v > 1, 0, undefined)",
        variance: "piecewise(v > 2, v/(v - 2), v > 1, inf, undefined)",
        median: "0",
        mode: "0",
        skewness: "piecewise(v > 3, 0, undefined)",
        kurtosis: "piecewise(v > 4, 3 + 6/(v - 4), v > 2, inf, undefined)",
        ..NONE
    },
    Dist {
        name: "betadist",
        valid: "a > 0 and b > 0",
        params: &[("a", ""), ("b", "")],
        support: ("0", "1"),
        inside: "0 < x and x < 1",
        pdf: "x^(a - 1)*(1 - x)^(b - 1)/beta(a, b)",
        cdf: "piecewise(x <= 0, 0, x < 1, betaregularized(x, a, b), 1)",
        mean: "a/(a + b)",
        variance: "a*b/((a + b)^2*(a + b + 1))",
        mode: "(a - 1)/(a + b - 2)",
        skewness: "2(b - a)*sqrt(a + b + 1)/((a + b + 2)*sqrt(a*b))",
        kurtosis: "3 + 6((a - b)^2*(a + b + 1) - a*b*(a + b + 2))/(a*b*(a + b + 2)*(a + b + 3))",
        ..NONE
    },
    Dist {
        name: "cauchydist",
        valid: "g > 0",
        params: &[("m", "0"), ("g", "1")],
        pdf: "1/(pi*g*(1 + ((x - m)/g)^2))",
        cdf: "1/2 + atan((x - m)/g)/pi",
        quantile: "m + g*tan(pi*(q - 1/2))",
        mean: "undefined",
        variance: "undefined",
        median: "m",
        mode: "m",
        skewness: "undefined",
        kurtosis: "undefined",
        ..NONE
    },
    Dist {
        name: "lognormaldist",
        valid: "s > 0",
        params: &[("m", "0"), ("s", "1")],
        support: ("0", "inf"),
        inside: "x > 0",
        pdf: "e^(-(ln(x) - m)^2/(2s^2))/(x*s*sqrt(2pi))",
        cdf: "piecewise(x > 0, (1 + erf((ln(x) - m)/(s*sqrt(2))))/2, 0)",
        quantile: "e^(m + s*sqrt(2)*erfinv(2q - 1))",
        mean: "e^(m + s^2/2)",
        variance: "(e^(s^2) - 1)*e^(2m + s^2)",
        median: "e^m",
        mode: "e^(m - s^2)",
        skewness: "(e^(s^2) + 2)*sqrt(e^(s^2) - 1)",
        kurtosis: "e^(4s^2) + 2e^(3s^2) + 3e^(2s^2) - 3",
        ..NONE
    },
    Dist {
        name: "weibulldist",
        valid: "k > 0 and l > 0",
        params: &[("k", ""), ("l", "1")],
        support: ("0", "inf"),
        inside: "x >= 0",
        pdf: "(k/l)*(x/l)^(k - 1)*e^(-(x/l)^k)",
        cdf: "piecewise(x >= 0, 1 - e^(-(x/l)^k), 0)",
        quantile: "l*(-ln(1 - q))^(1/k)",
        mean: "l*gamma(1 + 1/k)",
        variance: "l^2*(gamma(1 + 2/k) - gamma(1 + 1/k)^2)",
        median: "l*ln(2)^(1/k)",
        mode: "piecewise(k > 1, l*((k - 1)/k)^(1/k), 0)",
        ..NONE
    },
    Dist {
        name: "laplacedist",
        valid: "b > 0",
        params: &[("m", "0"), ("b", "1")],
        pdf: "e^(-abs(x - m)/b)/(2b)",
        cdf: "piecewise(x < m, e^((x - m)/b)/2, 1 - e^(-(x - m)/b)/2)",
        quantile: "piecewise(q < 1/2, m + b*ln(2q), m - b*ln(2 - 2q))",
        mean: "m",
        variance: "2b^2",
        median: "m",
        mode: "m",
        skewness: "0",
        kurtosis: "6",
        mgf: "e^(m*t)/(1 - b^2*t^2)",
        ..NONE
    },
    Dist {
        name: "rayleighdist",
        valid: "s > 0",
        params: &[("s", "1")],
        support: ("0", "inf"),
        inside: "x >= 0",
        pdf: "x*e^(-x^2/(2s^2))/s^2",
        cdf: "piecewise(x >= 0, 1 - e^(-x^2/(2s^2)), 0)",
        quantile: "s*sqrt(-2ln(1 - q))",
        mean: "s*sqrt(pi/2)",
        variance: "(4 - pi)*s^2/2",
        median: "s*sqrt(2ln(2))",
        mode: "s",
        ..NONE
    },
    Dist {
        name: "logisticdist",
        valid: "s > 0",
        params: &[("m", "0"), ("s", "1")],
        pdf: "e^(-(x - m)/s)/(s*(1 + e^(-(x - m)/s))^2)",
        cdf: "1/(1 + e^(-(x - m)/s))",
        quantile: "m + s*ln(q/(1 - q))",
        mean: "m",
        variance: "s^2*pi^2/3",
        median: "m",
        mode: "m",
        skewness: "0",
        kurtosis: "21/5",
        ..NONE
    },
    Dist {
        name: "paretodist",
        valid: "m > 0 and a > 0",
        params: &[("m", ""), ("a", "")],
        support: ("m", "inf"),
        inside: "x >= m",
        pdf: "a*m^a/x^(a + 1)",
        cdf: "piecewise(x >= m, 1 - (m/x)^a, 0)",
        quantile: "m*(1 - q)^(-1/a)",
        mean: "piecewise(a > 1, a*m/(a - 1), inf)",
        variance: "piecewise(a > 2, m^2*a/((a - 1)^2*(a - 2)), inf)",
        median: "m*2^(1/a)",
        mode: "m",
        ..NONE
    },
    Dist {
        name: "fdist",
        valid: "n > 0 and d > 0",
        params: &[("n", ""), ("d", "")],
        support: ("0", "inf"),
        inside: "x > 0",
        pdf: "sqrt((n*x)^n*d^d/(n*x + d)^(n + d))/(x*beta(n/2, d/2))",
        cdf: "piecewise(x > 0, betaregularized(n*x/(n*x + d), n/2, d/2), 0)",
        mean: "piecewise(d > 2, d/(d - 2), undefined)",
        variance: "piecewise(d > 4, 2d^2*(n + d - 2)/(n*(d - 2)^2*(d - 4)), undefined)",
        ..NONE
    },
    Dist {
        name: "binomialdist",
        valid: "n >= 0 and 0 <= p and p <= 1",
        params: &[("n", ""), ("p", "")],
        discrete: true,
        support: ("0", "n"),
        pdf: "binomial(n, x)*p^x*(1 - p)^(n - x)",
        cdf: "betaregularized(1 - p, n - floor(x), floor(x) + 1)",
        mean: "n*p",
        variance: "n*p*(1 - p)",
        mode: "floor((n + 1)*p)",
        skewness: "(1 - 2p)/sqrt(n*p*(1 - p))",
        kurtosis: "3 + (1 - 6p*(1 - p))/(n*p*(1 - p))",
        mgf: "(1 - p + p*e^t)^n",
        ..NONE
    },
    Dist {
        name: "poissondist",
        valid: "l > 0",
        params: &[("l", "")],
        discrete: true,
        support: ("0", "inf"),
        pdf: "l^x*e^(-l)/x!",
        cdf: "gammaq(floor(x) + 1, l)",
        mean: "l",
        variance: "l",
        mode: "floor(l)",
        skewness: "1/sqrt(l)",
        kurtosis: "3 + 1/l",
        mgf: "e^(l*(e^t - 1))",
        ..NONE
    },
    Dist {
        name: "geometricdist",
        valid: "0 < p and p <= 1",
        params: &[("p", "")],
        discrete: true,
        support: ("0", "inf"),
        pdf: "(1 - p)^x*p",
        cdf: "1 - (1 - p)^(floor(x) + 1)",
        mean: "(1 - p)/p",
        variance: "(1 - p)/p^2",
        mode: "0",
        skewness: "(2 - p)/sqrt(1 - p)",
        kurtosis: "9 + p^2/(1 - p)",
        mgf: "p/(1 - (1 - p)*e^t)",
        ..NONE
    },
    Dist {
        name: "bernoullidist",
        valid: "0 <= p and p <= 1",
        params: &[("p", "")],
        discrete: true,
        support: ("0", "1"),
        pdf: "p^x*(1 - p)^(1 - x)",
        cdf: "piecewise(x < 0, 0, x < 1, 1 - p, 1)",
        mean: "p",
        variance: "p*(1 - p)",
        skewness: "(1 - 2p)/sqrt(p*(1 - p))",
        kurtosis: "3 + (1 - 6p*(1 - p))/(p*(1 - p))",
        mgf: "1 - p + p*e^t",
        ..NONE
    },
    Dist {
        name: "discreteuniformdist",
        valid: "a <= b",
        params: &[("a", ""), ("b", "")],
        discrete: true,
        support: ("a", "b"),
        pdf: "1/(b - a + 1)",
        cdf: "(floor(x) - a + 1)/(b - a + 1)",
        mean: "(a + b)/2",
        variance: "((b - a + 1)^2 - 1)/12",
        median: "(a + b)/2",
        skewness: "0",
        ..NONE
    },
    Dist {
        name: "negativebinomialdist",
        valid: "r > 0 and 0 < p and p <= 1",
        params: &[("r", ""), ("p", "")],
        discrete: true,
        support: ("0", "inf"),
        pdf: "binomial(x + r - 1, x)*p^r*(1 - p)^x",
        cdf: "betaregularized(p, r, floor(x) + 1)",
        mean: "r*(1 - p)/p",
        variance: "r*(1 - p)/p^2",
        skewness: "(2 - p)/sqrt(r*(1 - p))",
        kurtosis: "3 + 6/r + p^2/(r*(1 - p))",
        mgf: "(p/(1 - (1 - p)*e^t))^r",
        ..NONE
    },
    Dist {
        name: "hypergeometricdist",
        valid: "0 <= n and n <= u and 0 <= s and s <= u",
        params: &[("n", ""), ("s", ""), ("u", "")],
        discrete: true,
        support: ("max(0, n + s - u)", "min(n, s)"),
        pdf: "binomial(s, x)*binomial(u - s, n - x)/binomial(u, n)",
        mean: "n*s/u",
        variance: "n*(s/u)*(1 - s/u)*(u - n)/(u - 1)",
        ..NONE
    },
];

thread_local! {
    static PARSED: RefCell<HashMap<&'static str, Expr>> = RefCell::new(HashMap::new());
}

fn formula(text: &'static str) -> Res {
    if let Some(hit) = PARSED.with(|p| p.borrow().get(text).cloned()) {
        return Ok(hit);
    }
    let e = crate::parser::parse(text)?
        .stmts
        .pop()
        .ok_or("an empty formula")?;
    PARSED.with(|p| p.borrow_mut().insert(text, e.clone()));
    Ok(e)
}

fn find(e: &Expr) -> Option<(&'static Dist, &[Expr])> {
    let Expr::Fn(name, args) = e else {
        return None;
    };
    DISTRIBUTIONS
        .iter()
        .find(|d| d.name == &**name)
        .map(|d| (d, &**args))
}

/// Whether the value is a distribution: `normaldist(0, 1)` and its kin.
/// A distribution with lists among its parameters stands for one of every entry: these,
/// when the lists are of one length.
pub(crate) fn spread(e: &Expr) -> Option<Result<Vec<Expr>, String>> {
    let (dist, given) = find(e)?;
    let size = given
        .iter()
        .filter_map(|v| v.as_list().map(<[Expr]>::len))
        .max()?;
    if given
        .iter()
        .any(|v| v.as_list().is_some_and(|l| l.len() != size))
    {
        return Some(Err(format!(
            "{}: the lists among the parameters are of one length",
            dist.name
        )));
    }
    let one = |k: usize| {
        let own = given.iter().map(|v| match v.as_list() {
            Some(l) => l[k].clone(),
            None => v.clone(),
        });
        Expr::call(dist.name, own.collect())
    };
    Some(Ok((0..size).map(one).collect()))
}

pub fn is_distribution(e: &Expr) -> bool {
    find(e).is_some()
}

pub fn is_distribution_name(name: &str) -> bool {
    DISTRIBUTIONS.iter().any(|d| d.name == name)
}

struct Bound<'a> {
    dist: &'static Dist,
    names: Vec<String>,
    values: Vec<Expr>,
    session: &'a mut Session,
}

impl Bound<'_> {
    /// The value of a formula of the table with the parameters and `extra` names set.
    fn value(&mut self, text: &'static str, extra: &[(&str, &Expr)]) -> Res {
        let raw = formula(text)?;
        let mut names = self.names.clone();
        let mut values = self.values.clone();
        for (n, v) in extra {
            names.push((*n).to_string());
            values.push((*v).clone());
        }
        self.session
            .with_bound(&names, values, |s| s.eval(&raw))
            .map(|v| simplify(&v))
    }

    fn ends(&mut self) -> Result<(Expr, Expr), String> {
        Ok((
            self.value(self.dist.support.0, &[])?,
            self.value(self.dist.support.1, &[])?,
        ))
    }

    fn missing(&self, what: &str) -> String {
        format!("the {what} of {} has no formula here", self.dist.name)
    }

    fn pdf(&mut self, x: &Expr) -> Res {
        let d = self.dist;
        if d.discrete {
            if x.is_const() {
                // a value off the support has no probability
                let (lo, hi) = self.ends()?;
                let whole = numeric::real_value(x, 30).is_some_and(|v| v.is_integer());
                let inside = crate::logic::truth(&Expr::rel(Rel::Le, lo, x.clone())) == Some(true)
                    && crate::logic::truth(&Expr::rel(Rel::Le, x.clone(), hi)) == Some(true);
                if !whole || !inside {
                    return Ok(Expr::zero());
                }
            }
            return self.value(d.pdf, &[("x", x)]);
        }
        let density = self.value(d.pdf, &[("x", x)]);
        if d.inside.is_empty() {
            return density;
        }
        let cond = self.value(d.inside, &[("x", x)])?;
        Ok(match crate::logic::truth(&cond) {
            Some(true) => density?,
            Some(false) => Expr::zero(),
            None => crate::funcs::func("piecewise", vec![cond, density?, Expr::zero()]),
        })
    }

    /// The sum of the probabilities of the whole numbers from the start of the support to `x`.
    fn discrete_sum(&mut self, x: &Expr) -> Option<Res> {
        let (lo, _) = self.ends().ok()?;
        let from = lo.as_i64()?;
        let to = numeric::real_value(x, 30)?
            .floor()
            .to_integer()
            .and_then(|v| v.to_i64())?;
        let most = if self.dist.cdf.is_empty() {
            100_000
        } else {
            1000
        };
        if !self.values.iter().all(|v| v.as_rat().is_some()) || to - from > most {
            return None;
        }
        let mut total = Vec::new();
        for k in from..=to {
            match self.pdf(&Expr::int(k)) {
                Ok(v) => total.push(v),
                Err(e) => return Some(Err(e)),
            }
        }
        Some(Ok(simplify(&add(total))))
    }

    fn cdf(&mut self, x: &Expr) -> Res {
        let d = self.dist;
        if x.is_inf() {
            return Ok(Expr::one());
        }
        if x.is_neg_inf() {
            return Ok(Expr::zero());
        }
        if d.discrete && x.is_const() {
            let (lo, hi) = self.ends()?;
            if crate::logic::truth(&Expr::rel(Rel::Lt, x.clone(), lo)) == Some(true) {
                return Ok(Expr::zero());
            }
            if crate::logic::truth(&Expr::rel(Rel::Ge, x.clone(), hi)) == Some(true) {
                return Ok(Expr::one());
            }
            if let Some(r) = self.discrete_sum(x) {
                return r;
            }
        }
        if d.cdf.is_empty() {
            return Err(self.missing("distribution function"));
        }
        self.value(d.cdf, &[("x", x)])
    }

    /// The smallest `x` with `cdf(x) >= q`, found by numbers.
    fn numeric_quantile(&mut self, q: &Expr) -> Res {
        let digits = numeric::digits();
        let bits = numeric::bits_for(digits) + 32;
        let (lo, hi) = self.ends()?;
        let at_end =
            |end: i64| crate::logic::truth(&Expr::eq(q.clone(), Expr::int(end))) == Some(true);
        if at_end(0) {
            return Ok(lo);
        }
        if at_end(1) {
            return Ok(hi);
        }
        // how far out in a tail the probability lies: the digits of the smaller one of q and
        // 1 - q that are zeros (1 - 10^-100 is no 1, though it rounds to it)
        let out = |v: &Expr| {
            numeric::real_value(v, 30)
                .filter(|v| *v > 0u32 && v.is_finite())
                .map(|v| -Float::with_val(64, v.log10_ref()).to_f64())
        };
        let complement = simplify(&sub(Expr::one(), q.clone()));
        let (Some(below), Some(above)) = (out(q), out(&complement)) else {
            return Err("quantile(dist, q): q must be a number from 0 to 1".into());
        };
        let tail = below.max(above).clamp(0.0, 1e6).ceil() as u32;
        let target = numeric::real_value(q, digits + 5 + tail)
            .filter(|v| *v > 0u32 && *v < 1u32)
            .ok_or("quantile(dist, q): q must be a number from 0 to 1")?;
        if self.dist.discrete {
            let from = lo.as_i64().ok_or_else(|| self.missing("quantile"))?;
            let mut total = Float::with_val(bits, 0);
            // (a probability that the sum reaches exactly is reached: the sum of rounded
            // numbers may end a hair below it)
            let reached = Float::with_val(bits, &target)
                * (1u32 - Float::with_val(bits, Float::i_pow_u(10, digits)).recip());
            let top = numeric::real_value(&hi, 30).map(|v| v.to_f64());
            // the probability of a whole number, ready to be worked out many times
            let place = Expr::sym("#k");
            let quick = self
                .value(self.dist.pdf, &[("x", &place)])
                .ok()
                .map(|body| crate::mfun::Numeric::new(&["#k".to_string()], &body, bits));
            let mass = |b: &mut Self, k: i64| -> Result<Float, String> {
                let fast = quick
                    .as_ref()
                    .and_then(|f| f.at(&[Float::with_val(bits, k)]).ok());
                if let Some(v) = fast {
                    return Ok(v);
                }
                let p = b.pdf(&Expr::int(k))?;
                numeric::real_value(&p, digits + 5).ok_or_else(|| b.missing("quantile"))
            };
            // the distribution function at a whole number, where it has a formula
            let known = !self.dist.cdf.is_empty();
            let below = |b: &mut Self, k: i64| -> Result<Float, String> {
                let v = b.value(b.dist.cdf, &[("x", &Expr::int(k))])?;
                numeric::real_value(&v, digits + 5).ok_or_else(|| b.missing("quantile"))
            };
            let number = |b: &mut Self, text: &'static str| -> Option<f64> {
                let v = b.value(text, &[]).ok()?;
                numeric::real_value(&v, 30)
                    .map(|v| v.to_f64())
                    .filter(|v| v.is_finite())
            };
            let middle = match self.dist.mean.is_empty() || self.dist.variance.is_empty() {
                true => None,
                false => number(self, self.dist.mean)
                    .zip(number(self, self.dist.variance))
                    .filter(|(_, v)| *v >= 0.0)
                    .map(|(m, v)| (m, v.sqrt())),
            };
            // the sum from the start of the support, unless the mass lies far above it
            let scan = 100_000i64;
            let far = known
                && middle
                    .is_some_and(|(m, s)| m - 14.0 * s > from as f64 + scan as f64 && m < 4e18);
            let end = match (far, known) {
                (true, _) => from,
                (false, true) => from + scan,
                (false, false) => from + 10_000_000,
            };
            for k in from..end {
                // (the last whole number of the support, also when its end is none)
                if top.is_some_and(|t| (k + 1) as f64 > t) {
                    return Ok(Expr::int(k));
                }
                total += mass(self, k)?;
                if total >= reached {
                    return Ok(Expr::int(k));
                }
                if hi.as_i64() == Some(k) {
                    return Ok(Expr::int(k));
                }
            }
            if !known {
                return Err(self.missing("quantile"));
            }
            // beyond that: between two whole numbers, by the distribution function
            let mut low = end - 1;
            let mut step = middle.map_or(1, |(_, s)| (s.ceil() as i64).max(1));
            if far && let Some((m, s)) = middle {
                let begin = (m - 14.0 * s).floor() as i64;
                if begin > low && below(self, begin)? < reached {
                    low = begin;
                }
            }
            let last = top.filter(|t| *t < 4e18).map(|t| t.floor() as i64);
            let mut high;
            loop {
                high = low
                    .checked_add(step)
                    .filter(|h| *h < 4_000_000_000_000_000_000)
                    .ok_or_else(|| self.missing("quantile"))?;
                if last.is_some_and(|t| high >= t) {
                    high = last.unwrap_or(high);
                    break;
                }
                if below(self, high)? >= reached {
                    break;
                }
                low = high;
                step = step.saturating_mul(2);
            }
            while high - low > 1 {
                let mid = low + (high - low) / 2;
                match below(self, mid)? >= reached {
                    true => high = mid,
                    false => low = mid,
                }
            }
            return Ok(Expr::int(high));
        }
        // The place by halving, with the distribution function worked out to more and more
        // digits until two places in a row agree: far out in a tail (and next to the middle
        // of an even distribution) the function has fewer digits than the place needs.
        // (with as many digits more from the start as the tail has zeros: fewer do not even
        // tell which side of the probability a place is on)
        let most = 64 + 4 * (digits + tail);
        let least = Float::with_val(64, Float::i_pow_u(10, digits + 2)).recip();
        let mut before: Option<Float> = None;
        let (mut more, mut step) = (tail, 16);
        loop {
            numeric::set_digits(digits + more);
            let found = self.place(q, &lo, &hi, digits + more, before.as_ref());
            numeric::set_digits(digits);
            let Some(place) = found? else {
                return Ok(Expr::zero());
            };
            let same = before.as_ref().is_some_and(|old| {
                Float::with_val(64, &place - old).abs()
                    <= Float::with_val(64, place.abs_ref()) * &least
            });
            if same {
                return Ok(numeric::to_float_expr(
                    rug::Complex::with_val(bits, place),
                    digits,
                ));
            }
            if more >= most {
                return Err(
                    "no quantile found: the distribution function has too few digits there".into(),
                );
            }
            more += step;
            step *= 2;
            before = Some(place);
        }
    }

    /// The smallest `x` with `cdf(x) >= q` of a continuous distribution between `lo` and
    /// `hi`, by halving, with numbers of `digits` digits; `near` is the place that fewer
    /// digits gave. `None`: 0 itself.
    fn place(
        &mut self,
        q: &Expr,
        lo: &Expr,
        hi: &Expr,
        digits: u32,
        near: Option<&Float>,
    ) -> Result<Option<Float>, String> {
        let bits = numeric::bits_for(digits) + 32;
        let target = numeric::real_value(q, digits + 5)
            .filter(|v| *v > 0u32 && *v < 1u32)
            .ok_or("quantile(dist, q): q must be a number from 0 to 1")?;
        let mut at = |x: &Float| -> Result<Float, String> {
            let point = numeric::to_float_expr(rug::Complex::with_val(bits, x), digits + 8);
            let v = self.cdf(&point)?;
            numeric::real_value(&v, digits + 5)
                .ok_or_else(|| "the distribution function has no number here".to_string())
        };
        let finite = |e: &Expr| numeric::real_value(e, 30).map(|v| Float::with_val(bits, v));
        let (low, high) = (finite(lo), finite(hi));
        // 0 itself, where the distribution reaches the probability there as far as its
        // digits tell (the middle of a distribution that is even, as Student's t): the
        // halving below would end a hair beside it, at a number made of rounding
        let inside = |end: &Option<Float>, below: bool| match end {
            Some(v) => (*v < 0u32) == below && !v.is_zero(),
            None => true,
        };
        if inside(&low, true)
            && inside(&high, false)
            && let Ok(middle) = at(&Float::new(bits))
        {
            let off = Float::with_val(bits, &middle - &target).abs();
            let share = Float::with_val(bits, Float::i_pow_u(10, digits + 3)).recip();
            if off <= Float::with_val(bits, &target * share) {
                return Ok(None);
            }
        }
        // A bracket with the distribution function less the probability at both ends: around
        // the place that fewer digits gave, where there is one
        let off = |v: Float| Float::with_val(bits, v - &target);
        let mut ends: Option<[Float; 4]> = None;
        if let Some(old) = near.filter(|old| !old.is_zero()) {
            let size = Float::with_val(bits, old.abs_ref());
            let mut step = Float::with_val(bits, &size >> 40u32);
            for _ in 0..6 {
                let mut a = Float::with_val(bits, old - &step);
                let mut b = Float::with_val(bits, old + &step);
                if let Some(end) = &low {
                    a = a.max(end);
                }
                if let Some(end) = &high {
                    b = b.min(end);
                }
                let fa = match low.as_ref() == Some(&a) {
                    true => Float::with_val(bits, -&target),
                    false => off(at(&a)?),
                };
                let fb = match high.as_ref() == Some(&b) {
                    true => Float::with_val(bits, 1u32 - Float::with_val(bits, &target)),
                    false => off(at(&b)?),
                };
                if fa < 0u32 && fb >= 0u32 {
                    ends = Some([a, fa, b, fb]);
                    break;
                }
                step <<= 8u32;
            }
        }
        // (or the ends of the support, or steps away from 0: twice as far each time, and
        // after some steps the square of it, as a tail like that of Student's t reaches out
        // to 10^120; the last place on the other side is the other end)
        let further = |v: Float, doubled: u32| -> Result<Float, String> {
            let limit = Float::with_val(bits, 1e300);
            let size = Float::with_val(bits, v.abs_ref());
            if size >= limit {
                return Err("no quantile found".into());
            }
            let next = match doubled < 8 || size < 2u32 {
                true => size * 2u32,
                false => size.square().min(&limit),
            };
            Ok(if v.is_sign_negative() { -next } else { next })
        };
        let [mut a, mut fa, mut b, mut fb] = match ends {
            Some(found) => found,
            None => {
                let mut above: Option<(Float, Float)> = None;
                let (mut a, mut fa) = match low {
                    Some(v) => (v, Float::with_val(bits, -&target)),
                    None => {
                        let mut v = Float::with_val(bits, -1);
                        let mut doubled = 0;
                        loop {
                            let f = off(at(&v)?);
                            if f <= 0u32 {
                                break (v, f);
                            }
                            above = Some((v.clone(), f));
                            v = further(v, doubled)?;
                            doubled += 1;
                        }
                    }
                };
                let (b, fb) = match (above, high) {
                    (Some(found), _) => found,
                    (None, Some(v)) => (
                        v,
                        Float::with_val(bits, 1u32 - Float::with_val(bits, &target)),
                    ),
                    (None, None) => {
                        let mut v = Float::with_val(bits, &a).abs() + 1u32;
                        let mut doubled = 0;
                        loop {
                            let f = off(at(&v)?);
                            if f >= 0u32 {
                                break (v, f);
                            }
                            (a, fa) = (v.clone(), f);
                            v = further(v, doubled)?;
                            doubled += 1;
                        }
                    }
                };
                [a, fa, b, fb]
            }
        };
        if fa.is_zero() {
            return Ok(Some(a));
        }
        // Between the ends: along the chord (with the value at an end that stays halved, so
        // that both ends move), and every third step by halving, which alone ends for sure
        let mut side = 0i8;
        for step in 1..=3 * (bits + 64) {
            let mid = Float::with_val(bits, &a + &b) / 2u32;
            if mid == a || mid == b {
                break;
            }
            // (the middle of two ends of one sign that are far apart is the root of their
            // product: the ends of a far tail differ by many powers of ten)
            let (near, far) =
                match Float::with_val(64, a.abs_ref()) < Float::with_val(64, b.abs_ref()) {
                    true => (&a, &b),
                    false => (&b, &a),
                };
            let apart = !near.is_zero()
                && near.is_sign_negative() == far.is_sign_negative()
                && Float::with_val(64, far / near) > 4u32;
            let mut c = match apart {
                true => {
                    let root = Float::with_val(bits, &a * &b).sqrt();
                    match a.is_sign_negative() {
                        true => -root,
                        false => root,
                    }
                }
                false => mid,
            };
            if step % 3 != 0 && !apart {
                let width = Float::with_val(bits, &b - &a);
                let chord = Float::with_val(
                    bits,
                    &b - Float::with_val(bits, &fb * width) / Float::with_val(bits, &fb - &fa),
                );
                if chord > a && chord < b {
                    c = chord;
                }
            }
            let fc = off(at(&c)?);
            if fc < 0u32 {
                (a, fa) = (c, fc);
                if side == -1 {
                    fb /= 2u32;
                }
                side = -1;
            } else {
                (b, fb) = (c, fc);
                if side == 1 {
                    fa /= 2u32;
                }
                side = 1;
            }
        }
        Ok(Some(b))
    }

    fn quantile(&mut self, q: &Expr) -> Res {
        let d = self.dist;
        let outside = |op: Rel, bound: Expr| {
            crate::logic::truth(&Expr::rel(op, q.clone(), bound)) == Some(true)
        };
        if q.is_const()
            && (numeric::real_value(q, 30).is_none()
                || outside(Rel::Lt, Expr::zero())
                || outside(Rel::Gt, Expr::one()))
        {
            return Err("quantile(dist, q): q must be a number from 0 to 1".into());
        }
        if !d.quantile.is_empty() {
            return self.value(d.quantile, &[("q", q)]);
        }
        if q.is_const() && self.values.iter().all(Expr::is_const) {
            return self.numeric_quantile(q);
        }
        Ok(Expr::call(
            "quantile",
            vec![Expr::call(d.name, self.values.clone()), q.clone()],
        ))
    }

    /// Runs `f` with the symbols among the parameters taken as what the distribution needs:
    /// places are real, the other parameters are positive.
    fn assumed<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        use crate::assume::{self, Facts};
        let mut touched: Vec<(String, Option<Facts>)> = Vec::new();
        for ((name, _), value) in self.dist.params.iter().zip(&self.values) {
            if let Expr::Sym(s) = value {
                let old = assume::all()
                    .iter()
                    .find(|(k, _)| **k == **s)
                    .map(|(_, facts)| *facts);
                let place = *name == "m" || self.dist.name.contains("uniform");
                assume::set(
                    s,
                    if place {
                        Facts::real()
                    } else {
                        Facts::positive()
                    },
                );
                touched.push((s.to_string(), old));
            }
        }
        let out = f(self);
        for (name, old) in touched {
            assume::clear(&name);
            if let Some(facts) = old {
                assume::set(&name, facts);
            }
        }
        out
    }

    /// The mean of `g(x)`: the integral or the sum of `g` times the density over the support.
    fn expectation(&mut self, g: &Expr, var: &str) -> Res {
        self.assumed(|b| b.expectation_inner(g, var))
    }

    fn expectation_inner(&mut self, g: &Expr, var: &str) -> Res {
        let x = Expr::sym(var);
        let (lo, hi) = self.ends()?;
        let weight = self.value(self.dist.pdf, &[("x", &x)])?;
        let f = simplify(&mul(vec![g.clone(), weight]));
        let digits = numeric::digits();
        if self.dist.discrete {
            let (v, approx) = crate::sums::sum(&f, var, &lo, &hi)?;
            return Ok(if approx { numeric::n(&v, digits) } else { v });
        }
        if f.free_syms().iter().any(|s| **s != *var) {
            return crate::integrate::definite_with_parameters(&f, var, &lo, &hi).ok_or_else(
                || {
                    format!(
                        "no closed form found for the mean of {g} over {}",
                        self.dist.name
                    )
                },
            );
        }
        let (v, approx) = crate::integrate::definite(&f, var, &lo, &hi)?;
        Ok(if approx { numeric::n(&v, digits) } else { v })
    }

    fn named(&mut self, text: &'static str, what: &str) -> Res {
        if text.is_empty() {
            return Err(self.missing(what));
        }
        self.value(text, &[])
    }
}

impl Session {
    fn bound<'a>(&'a mut self, e: &Expr) -> Option<Result<Bound<'a>, String>> {
        let (dist, given) = find(e)?;
        if given.len() > dist.params.len() {
            return Some(Err(format!(
                "{} takes {} parameter(s) at most",
                dist.name,
                dist.params.len()
            )));
        }
        let mut values = Vec::new();
        for (k, (name, default)) in dist.params.iter().enumerate() {
            match given.get(k) {
                Some(v) => values.push(v.clone()),
                None if default.is_empty() => {
                    return Some(Err(format!(
                        "{}: the parameter {name} is missing ({} of {} given)",
                        dist.name,
                        given.len(),
                        dist.params.len()
                    )));
                }
                None => match formula(default) {
                    Ok(v) => values.push(v),
                    Err(e) => return Some(Err(e)),
                },
            }
        }
        for (value, (name, _)) in values.iter().zip(dist.params) {
            let odd = value.as_list().is_some()
                || crate::radix::text_of(value).is_some()
                || matches!(value, Expr::Bool(_) | Expr::Rel(..))
                || value.has_sym(crate::expr::UNDEF)
                || (value.is_const()
                    && !value.is_inf()
                    && !value.is_neg_inf()
                    && numeric::real_value(value, 30).is_none());
            if odd {
                return Some(Err(format!(
                    "{}: the parameter {name} is a real number or a name, not {value}",
                    dist.name
                )));
            }
        }
        let mut bound = Bound {
            dist,
            names: dist.params.iter().map(|(n, _)| (*n).to_string()).collect(),
            values,
            session: self,
        };
        // parameters that are numbers must be ones that the distribution has
        if !dist.valid.is_empty()
            && let Ok(holds) = bound.value(dist.valid, &[])
            && crate::logic::truth(&holds) == Some(false)
        {
            let names: Vec<&str> = dist.params.iter().map(|(n, _)| *n).collect();
            return Some(Err(format!(
                "{}({}) needs {}",
                dist.name,
                names.join(", "),
                dist.valid
            )));
        }
        Some(Ok(bound))
    }

    /// An error when the parameters of a distribution are none that it has.
    pub(crate) fn valid_distribution(&mut self, e: &Expr) -> Result<(), String> {
        match self.bound(e) {
            Some(Err(e)) => Err(e),
            _ => Ok(()),
        }
    }

    /// The commands whose first argument, or last for `expectation` and `probability`, is a
    /// distribution.
    pub(crate) fn distribution(&mut self, name: &str, a: &[Expr]) -> Option<Res> {
        let which = match (name, a.len()) {
            ("expectation" | "probability", 3) => 2,
            (_, 1 | 2) => 0,
            _ => return None,
        };
        if let Some(parts) = spread(&a[which]) {
            return Some(parts.and_then(|parts| {
                let mut out = Vec::with_capacity(parts.len());
                for part in parts {
                    let mut own = a.to_vec();
                    own[which] = part;
                    out.push(self.call(name, &own)?);
                }
                Ok(Expr::list(out))
            }));
        }
        let mut b = match self.bound(&a[which])? {
            Ok(b) => b,
            Err(e) => return Some(Err(e)),
        };
        let d = b.dist;
        Some(match (name, a.len()) {
            ("pdf", 2) => b.pdf(&a[1]),
            ("cdf", 2) => b.cdf(&a[1]),
            ("quantile", 2) => match &a[1] {
                Expr::List(qs) => qs
                    .iter()
                    .map(|q| b.quantile(q))
                    .collect::<Result<Vec<_>, _>>()
                    .map(Expr::list),
                q => b.quantile(q),
            },
            ("avg", 1) => b.named(d.mean, "mean"),
            ("variance", 1) => b.named(d.variance, "variance"),
            ("stddev", 1) => b.assumed(|b| {
                b.named(d.variance, "variance")
                    .map(|v| simplify(&crate::canon::sqrt(v)))
            }),
            ("median", 1) => {
                if d.median.is_empty() {
                    b.quantile(&Expr::half())
                } else {
                    b.named(d.median, "median")
                }
            }
            ("mode", 1) => b.named(d.mode, "mode"),
            ("skewness", 1) => b.named(d.skewness, "skewness"),
            ("kurtosis", 1) => b.named(d.kurtosis, "kurtosis"),
            ("mgf", 2) => {
                if d.mgf.is_empty() {
                    let x = Expr::sym("dist_x");
                    let g = crate::canon::pow(Expr::e(), mul(vec![a[1].clone(), x]));
                    b.expectation(&g, "dist_x")
                } else {
                    b.value(d.mgf, &[("t", &a[1])])
                }
            }
            ("moment", 2) | ("centralmoment", 2) => (|| {
                let k = a[1]
                    .as_i64()
                    .filter(|k| (0..=200).contains(k))
                    .ok_or("the order of a moment must be a whole number from 0 to 200")?;
                let x = Expr::sym("dist_x");
                let base = if name == "moment" {
                    x
                } else {
                    sub(x, b.named(d.mean, "mean")?)
                };
                b.expectation(&crate::canon::powi(base, k), "dist_x")
            })(),
            ("expectation", 3) => match a[1].sym_name() {
                Some(var) => b.expectation(&a[0], var),
                None => Err("expectation(expr, x, dist)".to_string()),
            },
            ("probability", 3) => (|| {
                let usage = "probability(x < c, x, dist), with <, <=, >, >=, = or a < x < b";
                let var = a[1].sym_name().ok_or(usage)?;
                let rels: Vec<&Expr> = match &a[0] {
                    Expr::Fn(n, items) if &**n == "and" => items.iter().collect(),
                    other => vec![other],
                };
                // the event as an interval: lower end, upper end, each open or closed
                let mut low: Option<(Expr, bool)> = None;
                let mut high: Option<(Expr, bool)> = None;
                let mut point: Option<Expr> = None;
                for r in rels {
                    let Expr::Rel(op, l, r) = r else {
                        return Err(usage.to_string());
                    };
                    let (op, c) = if l.is_sym(var) && !r.has_sym(var) {
                        (*op, (**r).clone())
                    } else if r.is_sym(var) && !l.has_sym(var) {
                        (op.flip(), (**l).clone())
                    } else {
                        return Err(usage.to_string());
                    };
                    match op {
                        Rel::Lt => high = Some((c, false)),
                        Rel::Le => high = Some((c, true)),
                        Rel::Gt => low = Some((c, false)),
                        Rel::Ge => low = Some((c, true)),
                        Rel::Eq => point = Some(c),
                        Rel::Ne => return Err(usage.to_string()),
                    }
                }
                if let Some(c) = point {
                    return if d.discrete {
                        b.pdf(&c)
                    } else {
                        Ok(Expr::zero())
                    };
                }
                // P(x <= c), or P(x < c): for whole values that is P(x <= ceil(c) - 1)
                let mut upto = |c: &Expr, closed: bool| -> Res {
                    if d.discrete && !closed {
                        let before = sub(crate::funcs::func("ceil", vec![c.clone()]), Expr::one());
                        b.cdf(&simplify(&before))
                    } else {
                        b.cdf(c)
                    }
                };
                let top = match &high {
                    Some((c, closed)) => upto(c, *closed)?,
                    None => Expr::one(),
                };
                let bottom = match &low {
                    Some((c, closed)) => upto(c, !*closed)?,
                    None => Expr::zero(),
                };
                Ok(simplify(&sub(top, bottom)))
            })(),
            _ => return None,
        })
    }
}

/// A distribution with its parameters checked: the count only, the values may be symbols.
pub fn construct(name: &str, a: &[Expr]) -> Option<Res> {
    let d = DISTRIBUTIONS.iter().find(|d| d.name == name)?;
    if a.len() > d.params.len() {
        return Some(Err(format!(
            "{name} takes {} parameter(s) at most, {} given",
            d.params.len(),
            a.len()
        )));
    }
    let _ = div;
    Some(Ok(Expr::call(name, a.to_vec())))
}
