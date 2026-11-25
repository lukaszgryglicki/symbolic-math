//! Settings of a session that are written as statements: `digits = 50`, `roots = 10`,
//! `terms = 5000000`, `verbose`. The parser reads one at the start of a statement (`setting`
//! with the name and the value); what follows it on the line is the next statement.

use std::cell::Cell;

thread_local! {
    static ROOTS: Cell<Option<u32>> = const { Cell::new(None) };
    static TERMS: Cell<Option<u64>> = const { Cell::new(None) };
    static SOLVING: Cell<Solving> = const { Cell::new(Solving::Inside) };
}

/// For whom an equation is solved.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Solving {
    /// An equation that the user wrote: `roots = N` holds for it, and a numeric search lists
    /// the roots nearest to 0.
    Listed,
    /// An equation that a method solves on its way: no setting of the user changes it.
    Inside,
    /// An equation whose every real root counts: the ends of an inequality, the poles of an
    /// integrand, the turning points of a function. A numeric search goes along the real
    /// line - between the two numbers when they are given - for all the roots that are
    /// there, and none is left out.
    Thorough(Option<(f64, f64)>),
}

fn solving_as<T>(kind: Solving, f: impl FnOnce() -> T) -> T {
    let before = SOLVING.replace(kind);
    let out = f();
    SOLVING.set(before);
    out
}

/// Runs `f` as the solving of an equation of the user (`Solving::Listed`).
pub fn listing<T>(f: impl FnOnce() -> T) -> T {
    solving_as(Solving::Listed, f)
}

/// Runs `f` as the solving of an equation whose every real root counts, between the two
/// numbers of `range` when it has them (`Solving::Thorough`).
pub fn thorough<T>(range: Option<(f64, f64)>, f: impl FnOnce() -> T) -> T {
    solving_as(Solving::Thorough(range), f)
}

pub fn solving() -> Solving {
    SOLVING.get()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Setting {
    Digits,
    Roots,
    Terms,
    Verbose,
    /// `debug`: verbose with every detail.
    Debug,
    /// `settings`: the list of them all.
    List,
}

pub const MAX_DIGITS: u32 = 1_000_000;
pub const MAX_ROOTS: u32 = 100_000;
pub const MAX_TERMS: u64 = 1_000_000_000_000;

/// The setting that a word names.
pub fn named(word: &str) -> Option<Setting> {
    Some(match word {
        "digits" | "prec" | "precision" => Setting::Digits,
        "roots" | "maxroots" | "solutions" => Setting::Roots,
        "terms" | "maxterms" => Setting::Terms,
        "verbose" => Setting::Verbose,
        "debug" => Setting::Debug,
        "settings" => Setting::List,
        _ => return None,
    })
}

/// The first name of a setting.
pub fn name(setting: Setting) -> &'static str {
    match setting {
        Setting::Digits => "digits",
        Setting::Roots => "roots",
        Setting::Terms => "terms",
        Setting::Verbose => "verbose",
        Setting::Debug => "debug",
        Setting::List => "settings",
    }
}

/// A setting that stands alone, without a value: `verbose`, `debug`, `settings`.
pub fn bare(setting: Setting) -> bool {
    matches!(setting, Setting::Verbose | Setting::Debug | Setting::List)
}

/// A word that is the value of a setting.
pub fn value_word(word: &str) -> bool {
    matches!(
        word,
        "default" | "auto" | "on" | "off" | "all" | "none" | "yes" | "no" | "true" | "false"
    )
}

/// How many roots a numeric search is asked for by `roots = N`: for an equation of the user
/// only (see `Solving`); `None` leaves each search its own way.
pub fn roots() -> Option<u32> {
    match solving() {
        Solving::Listed => ROOTS.get(),
        _ => None,
    }
}

/// `roots = N` as it was set.
pub fn roots_set() -> Option<u32> {
    ROOTS.get()
}

pub fn set_roots(n: Option<u32>) {
    ROOTS.set(n.map(|n| n.clamp(1, MAX_ROOTS)));
}

/// The most terms of a sum, of a product or of a series before the method gives up: what was
/// set, or else `usual`, the limit of the method itself.
pub fn terms(usual: u64) -> u64 {
    TERMS.get().unwrap_or(usual)
}

pub fn terms_set() -> Option<u64> {
    TERMS.get()
}

pub fn set_terms(n: Option<u64>) {
    TERMS.set(n.map(|n| n.clamp(1, MAX_TERMS)));
}

/// Every setting as it is at the start of a session.
pub fn reset() {
    ROOTS.set(None);
    TERMS.set(None);
    crate::trace::set_level(0);
}

/// `name = value` of one setting, as `settings` lists it. `digits` is what was asked for.
pub fn shown(setting: Setting, digits: Option<u32>) -> String {
    match setting {
        Setting::Digits => match digits {
            Some(d) => format!("digits = {d}"),
            None => {
                "digits = default (32, or as many as the longest number of an input)".to_string()
            }
        },
        Setting::Roots => match roots_set() {
            Some(n) => format!("roots = {n}"),
            None => "roots = default (a numeric search lists what its usual starting points find)"
                .to_string(),
        },
        Setting::Terms => match terms_set() {
            Some(n) => format!("terms = {n}"),
            None => "terms = default (each sum and each series has its own limit)".to_string(),
        },
        Setting::Verbose | Setting::Debug => format!(
            "verbose = {}",
            match crate::trace::level() {
                0 => "off",
                1 => "on",
                _ => "all",
            }
        ),
        Setting::List => String::new(),
    }
}

/// The lines of `settings`.
pub fn listed(digits: Option<u32>) -> Vec<String> {
    [
        Setting::Digits,
        Setting::Roots,
        Setting::Terms,
        Setting::Verbose,
    ]
    .iter()
    .map(|s| shown(*s, digits))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_words_of_settings_and_their_values() {
        for (word, setting) in [
            ("digits", Setting::Digits),
            ("prec", Setting::Digits),
            ("precision", Setting::Digits),
            ("roots", Setting::Roots),
            ("maxroots", Setting::Roots),
            ("solutions", Setting::Roots),
            ("terms", Setting::Terms),
            ("maxterms", Setting::Terms),
            ("verbose", Setting::Verbose),
            ("debug", Setting::Debug),
            ("settings", Setting::List),
        ] {
            assert_eq!(named(word), Some(setting), "{word}");
            assert_eq!(named(name(setting)), Some(setting));
        }
        assert_eq!(named("Digits"), None);
        assert_eq!(named("root"), None);
        assert!(bare(Setting::Verbose) && bare(Setting::Debug) && bare(Setting::List));
        assert!(!bare(Setting::Digits) && !bare(Setting::Roots) && !bare(Setting::Terms));
        assert!(value_word("default") && value_word("off") && !value_word("maybe"));
    }

    #[test]
    fn what_is_set_is_what_the_methods_see() {
        reset();
        assert_eq!(
            (roots_set(), terms_set(), terms(2_000_000)),
            (None, None, 2_000_000)
        );
        set_roots(Some(7));
        set_terms(Some(500));
        assert_eq!(
            (roots_set(), terms_set(), terms(2_000_000)),
            (Some(7), Some(500), 500)
        );
        // (out of range: the nearest value that is allowed)
        set_roots(Some(0));
        set_terms(Some(0));
        assert_eq!((roots_set(), terms_set()), (Some(1), Some(1)));
        set_roots(Some(u32::MAX));
        set_terms(Some(u64::MAX));
        assert_eq!(
            (roots_set(), terms_set()),
            (Some(MAX_ROOTS), Some(MAX_TERMS))
        );
        assert_eq!(
            listed(Some(40))[..3],
            ["digits = 40", "roots = 100000", "terms = 1000000000000"]
        );
        reset();
        assert_eq!(
            listed(None),
            [
                "digits = default (32, or as many as the longest number of an input)",
                "roots = default (a numeric search lists what its usual starting points find)",
                "terms = default (each sum and each series has its own limit)",
                "verbose = off"
            ]
        );
    }

    #[test]
    fn the_count_of_roots_holds_for_an_equation_of_the_user_only() {
        reset();
        set_roots(Some(3));
        // (an equation that a method solves on its way is not cut to that count)
        assert_eq!((solving(), roots()), (Solving::Inside, None));
        listing(|| {
            assert_eq!((solving(), roots()), (Solving::Listed, Some(3)));
            // the ends of an inequality inside it: every real root
            thorough(Some((0.0, 2.0)), || {
                assert_eq!(solving(), Solving::Thorough(Some((0.0, 2.0))));
                assert_eq!(roots(), None);
            });
            assert_eq!((solving(), roots()), (Solving::Listed, Some(3)));
        });
        thorough(None, || {
            assert_eq!((solving(), roots()), (Solving::Thorough(None), None));
            listing(|| assert_eq!(roots(), Some(3)));
        });
        assert_eq!(
            (solving(), roots(), roots_set()),
            (Solving::Inside, None, Some(3))
        );
        reset();
    }
}
