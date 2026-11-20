//! The distribution functions of MATLAB by their names: `normpdf`, `tcdf`, `chi2inv`,
//! `poissrnd`, `binostat` and their kin are the distributions of this program in another dress.

use crate::expr::Expr;

struct Family {
    /// What the names start with.
    stem: &'static str,
    /// The name MATLAB gives the distribution in pdf("Name", ...).
    title: &'static str,
    /// The values of the parameters that may be left out, the last ones first to go.
    defaults: &'static [i64],
    /// How many parameters there are at most.
    params: usize,
}

const FAMILIES: &[Family] = &[
    Family {
        stem: "norm",
        title: "normal",
        defaults: &[0, 1],
        params: 2,
    },
    Family {
        stem: "unif",
        title: "uniform",
        defaults: &[0, 1],
        params: 2,
    },
    Family {
        stem: "exp",
        title: "exponential",
        defaults: &[1],
        params: 1,
    },
    Family {
        stem: "gam",
        title: "gamma",
        defaults: &[1],
        params: 2,
    },
    Family {
        stem: "beta",
        title: "beta",
        defaults: &[],
        params: 2,
    },
    Family {
        stem: "chi2",
        title: "chisquare",
        defaults: &[],
        params: 1,
    },
    Family {
        stem: "t",
        title: "t",
        defaults: &[],
        params: 1,
    },
    Family {
        stem: "f",
        title: "f",
        defaults: &[],
        params: 2,
    },
    Family {
        stem: "logn",
        title: "lognormal",
        defaults: &[0, 1],
        params: 2,
    },
    Family {
        stem: "wbl",
        title: "weibull",
        defaults: &[1, 1],
        params: 2,
    },
    Family {
        stem: "rayl",
        title: "rayleigh",
        defaults: &[],
        params: 1,
    },
    Family {
        stem: "bino",
        title: "binomial",
        defaults: &[],
        params: 2,
    },
    Family {
        stem: "poiss",
        title: "poisson",
        defaults: &[],
        params: 1,
    },
    Family {
        stem: "geo",
        title: "geometric",
        defaults: &[],
        params: 1,
    },
    Family {
        stem: "nbin",
        title: "negativebinomial",
        defaults: &[],
        params: 2,
    },
    Family {
        stem: "hyge",
        title: "hypergeometric",
        defaults: &[],
        params: 3,
    },
    Family {
        stem: "unid",
        title: "discreteuniform",
        defaults: &[],
        params: 1,
    },
];

const FORMS: &[&str] = &["pdf", "cdf", "inv", "rnd", "stat"];

fn split(name: &str) -> Option<(&'static Family, &'static str)> {
    FORMS.iter().find_map(|form| {
        let stem = name.strip_suffix(form)?;
        FAMILIES.iter().find(|f| f.stem == stem).map(|f| (f, *form))
    })
}

/// Whether a name is one of these functions.
pub fn is_name(name: &str) -> bool {
    split(name).is_some()
}

/// The distribution of a family with the parameters as MATLAB orders them.
fn distribution(family: &Family, mut params: Vec<Expr>, name: &str) -> Result<Expr, String> {
    let least = family.params - family.defaults.len();
    if params.len() < least || params.len() > family.params {
        return Err(format!(
            "{name}: the distribution takes {} parameter(s), {} given",
            if least == family.params {
                format!("{least}")
            } else {
                format!("{least} to {}", family.params)
            },
            params.len()
        ));
    }
    while params.len() < family.params {
        let k = params.len() - least;
        params.push(Expr::int(family.defaults[k]));
    }
    let call = |n: &str, a: Vec<Expr>| Expr::call(n, a);
    let mut p = params.into_iter();
    let mut next = || p.next().unwrap_or_else(Expr::one);
    Ok(match family.stem {
        "norm" => call("normaldist", vec![next(), next()]),
        "unif" => call("uniformdist", vec![next(), next()]),
        "exp" => call(
            "exponentialdist",
            vec![Expr::Pow(next().into(), Expr::minus_one().into())],
        ),
        "gam" => call("gammadist", vec![next(), next()]),
        "beta" => call("betadist", vec![next(), next()]),
        "chi2" => call("chisquaredist", vec![next()]),
        "t" => call("studenttdist", vec![next()]),
        "f" => call("fdist", vec![next(), next()]),
        "logn" => call("lognormaldist", vec![next(), next()]),
        "wbl" => {
            let (scale, shape) = (next(), next());
            call("weibulldist", vec![shape, scale])
        }
        "rayl" => call("rayleighdist", vec![next()]),
        "bino" => call("binomialdist", vec![next(), next()]),
        "poiss" => call("poissondist", vec![next()]),
        "geo" => call("geometricdist", vec![next()]),
        "nbin" => call("negativebinomialdist", vec![next(), next()]),
        "hyge" => {
            let (all, marked, drawn) = (next(), next(), next());
            call("hypergeometricdist", vec![drawn, marked, all])
        }
        _ => call("discreteuniformdist", vec![Expr::one(), next()]),
    })
}

fn build(family: &Family, form: &str, mut a: Vec<Expr>, name: &str) -> Result<Expr, String> {
    let call = |n: &str, a: Vec<Expr>| Expr::call(n, a);
    match form {
        "rnd" => Ok(call("random", vec![distribution(family, a, name)?])),
        "stat" => {
            let d = distribution(family, a, name)?;
            Ok(Expr::list(vec![
                call("avg", vec![d.clone()]),
                call("variance", vec![d]),
            ]))
        }
        _ => {
            if a.is_empty() {
                return Err(format!("{name} needs the place of the value first"));
            }
            let upper = form == "cdf"
                && a.last()
                    .and_then(crate::radix::text_of)
                    .is_some_and(|t| t.eq_ignore_ascii_case("upper"));
            if upper {
                a.pop();
            }
            let at = a.remove(0);
            let d = distribution(family, a, name)?;
            Ok(match form {
                "pdf" => call("pdf", vec![d, at]),
                "inv" => call("quantile", vec![d, at]),
                _ if upper => Expr::Add(
                    vec![
                        Expr::one(),
                        Expr::Mul(vec![Expr::minus_one(), call("cdf", vec![d, at])].into()),
                    ]
                    .into(),
                ),
                _ => call("cdf", vec![d, at]),
            })
        }
    }
}

/// The call of one of these functions as a call of this program's distributions.
pub fn rewrite(name: &str, a: Vec<Expr>) -> Result<Expr, String> {
    let (family, form) =
        split(name).ok_or_else(|| format!("{name} is no distribution function"))?;
    build(family, form, a, name)
}

/// The distribution that MATLAB names so, with the parameters as MATLAB orders them.
pub(crate) fn named(title: &str, params: Vec<Expr>, name: &str) -> Option<Result<Expr, String>> {
    let title: String = title
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    let family = FAMILIES.iter().find(|f| f.title == title)?;
    Some(distribution(family, params, name))
}

/// `pdf("Normal", x, mu, sigma)` and its kin: a distribution named by a text.
pub fn by_title(command: &str, a: &[Expr]) -> Option<Result<Expr, String>> {
    let title: String = crate::radix::text_of(a.first()?)?
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    let family = FAMILIES.iter().find(|f| f.title == title)?;
    let form = match command {
        "pdf" => "pdf",
        "cdf" => "cdf",
        "quantile" => "inv",
        "random" => "rnd",
        _ => return None,
    };
    Some(build(family, form, a[1..].to_vec(), command))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_name_splits_into_a_family_and_a_form() {
        for f in FAMILIES {
            for form in FORMS {
                let name = format!("{}{form}", f.stem);
                let (family, found) = split(&name).expect("a known name");
                assert_eq!((family.stem, found), (f.stem, *form));
                assert!(
                    crate::names::function(&name).is_some() || name == "finv",
                    "{name}"
                );
            }
        }
        assert!(!is_name("pdf") && !is_name("inv") && !is_name("stat"));
    }
}
