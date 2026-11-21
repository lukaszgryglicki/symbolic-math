//! Quantities with units: conversion between units, SI base units, dimensions.
//!
//! Unit names are read inside the unit commands only (`convert`, `siunits`, `dimension`,
//! `quantity`); elsewhere `m`, `s` or `J` are symbols like any other. A quantity is the node
//! `quantity(value, unit)`, printed as `value unit`.

use crate::canon::{add, div, map_bottom_up, mul, pow, sub};
use crate::expr::Expr;
use crate::funcs::func;
use crate::num::{Rat, parse_decimal};
use crate::{numeric, simplify};
use rug::Complex;
use rug::ops::Pow;
use std::cell::RefCell;
use std::collections::HashMap;

/// Powers of metre, kilogram, second, ampere, kelvin, mole, candela and bit.
type Dim = [Rat; 8];

const BASE: [&str; 8] = ["m", "kg", "s", "A", "K", "mol", "cd", "bit"];
const KINDS: [&str; 8] = [
    "length",
    "mass",
    "time",
    "current",
    "temperature",
    "amount",
    "luminosity",
    "information",
];

struct Def {
    /// The symbol first, then the other spellings.
    names: &'static str,
    dim: [i8; 8],
    /// The size in SI base units, as an expression.
    size: &'static str,
    /// Takes the SI prefixes.
    prefixes: bool,
    /// For a temperature scale: the reading of absolute zero, negated (in its own degrees).
    zero: &'static str,
    /// For a measured constant: how many digits are known.
    digits: u32,
}

const fn unit(names: &'static str, dim: [i8; 8], size: &'static str) -> Def {
    Def {
        names,
        dim,
        size,
        prefixes: false,
        zero: "",
        digits: 0,
    }
}

/// A unit that takes the SI prefixes.
const fn si(names: &'static str, dim: [i8; 8], size: &'static str) -> Def {
    Def {
        names,
        dim,
        size,
        prefixes: true,
        zero: "",
        digits: 0,
    }
}

/// A temperature scale with its zero away from absolute zero.
const fn scale(names: &'static str, size: &'static str, zero: &'static str) -> Def {
    Def {
        names,
        dim: TEMPERATURE,
        size,
        prefixes: false,
        zero,
        digits: 0,
    }
}

/// A measured constant, known to `digits` digits.
const fn measured(names: &'static str, dim: [i8; 8], size: &'static str, digits: u32) -> Def {
    Def {
        names,
        dim,
        size,
        prefixes: false,
        zero: "",
        digits,
    }
}

const NONE: [i8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];
const LENGTH: [i8; 8] = [1, 0, 0, 0, 0, 0, 0, 0];
const AREA: [i8; 8] = [2, 0, 0, 0, 0, 0, 0, 0];
const VOLUME: [i8; 8] = [3, 0, 0, 0, 0, 0, 0, 0];
const MASS: [i8; 8] = [0, 1, 0, 0, 0, 0, 0, 0];
const TIME: [i8; 8] = [0, 0, 1, 0, 0, 0, 0, 0];
const CURRENT: [i8; 8] = [0, 0, 0, 1, 0, 0, 0, 0];
const TEMPERATURE: [i8; 8] = [0, 0, 0, 0, 1, 0, 0, 0];
const AMOUNT: [i8; 8] = [0, 0, 0, 0, 0, 1, 0, 0];
const LIGHT: [i8; 8] = [0, 0, 0, 0, 0, 0, 1, 0];
const DATA: [i8; 8] = [0, 0, 0, 0, 0, 0, 0, 1];
const RATE: [i8; 8] = [0, 0, -1, 0, 0, 0, 0, 0];
const SPEED: [i8; 8] = [1, 0, -1, 0, 0, 0, 0, 0];
const ACCELERATION: [i8; 8] = [1, 0, -2, 0, 0, 0, 0, 0];
const FORCE: [i8; 8] = [1, 1, -2, 0, 0, 0, 0, 0];
const PRESSURE: [i8; 8] = [-1, 1, -2, 0, 0, 0, 0, 0];
const ENERGY: [i8; 8] = [2, 1, -2, 0, 0, 0, 0, 0];
const POWER: [i8; 8] = [2, 1, -3, 0, 0, 0, 0, 0];
const CHARGE: [i8; 8] = [0, 0, 1, 1, 0, 0, 0, 0];
const VOLTAGE: [i8; 8] = [2, 1, -3, -1, 0, 0, 0, 0];
const RESISTANCE: [i8; 8] = [2, 1, -3, -2, 0, 0, 0, 0];
const CAPACITANCE: [i8; 8] = [-2, -1, 4, 2, 0, 0, 0, 0];
const INDUCTANCE: [i8; 8] = [2, 1, -2, -2, 0, 0, 0, 0];
const FLUX: [i8; 8] = [2, 1, -2, -1, 0, 0, 0, 0];
const FLUX_DENSITY: [i8; 8] = [0, 1, -2, -1, 0, 0, 0, 0];
const DOSE: [i8; 8] = [2, 0, -2, 0, 0, 0, 0, 0];

const LBF: &str = "4.4482216152605";
const GALLON: &str = "0.003785411784";

const UNITS: &[Def] = &[
    // length
    si("m meter metre meters metres", LENGTH, "1"),
    unit("inch in inches", LENGTH, "0.0254"),
    unit("ft foot feet", LENGTH, "0.3048"),
    unit("yd yard yards", LENGTH, "0.9144"),
    unit("mi mile miles", LENGTH, "1609.344"),
    unit("nmi nauticalmile nauticalmiles", LENGTH, "1852"),
    unit("au AU astronomicalunit", LENGTH, "149597870700"),
    si("ly lightyear lightyears", LENGTH, "9460730472580800"),
    si("pc parsec parsecs", LENGTH, "149597870700*648000/pi"),
    unit("angstrom angstroms", LENGTH, "1e-10"),
    unit("micron microns", LENGTH, "1e-6"),
    unit("mil thou", LENGTH, "0.0000254"),
    unit("fathom fathoms", LENGTH, "1.8288"),
    unit("furlong furlongs", LENGTH, "201.168"),
    // mass
    si("g gram grams gramme grammes", MASS, "0.001"),
    si("t tonne tonnes", MASS, "1000"),
    unit("lb lbs pound pounds", MASS, "0.45359237"),
    unit("oz ounce ounces", MASS, "0.45359237/16"),
    unit("st stone stones", MASS, "6.35029318"),
    unit("ton tons shortton", MASS, "907.18474"),
    unit("longton", MASS, "1016.0469088"),
    unit("grain grains", MASS, "0.00006479891"),
    unit("ct carat carats", MASS, "0.0002"),
    unit("slug slugs", MASS, "4.4482216152605/0.3048"),
    measured("u Da dalton amu", MASS, "1.66053906892e-27", 12),
    // time
    si("s sec second seconds", TIME, "1"),
    unit("min minute minutes", TIME, "60"),
    unit("h hr hour hours", TIME, "3600"),
    unit("d day days", TIME, "86400"),
    unit("wk week weeks", TIME, "604800"),
    unit("fortnight fortnights", TIME, "1209600"),
    si("yr year years", TIME, "31557600"),
    // the other base units
    si("A amp amps ampere amperes", CURRENT, "1"),
    si("K kelvin kelvins", TEMPERATURE, "1"),
    scale("degC celsius", "1", "273.15"),
    scale("degF fahrenheit", "5/9", "459.67"),
    unit("degR rankine", TEMPERATURE, "5/9"),
    si("mol mole moles", AMOUNT, "1"),
    si("cd candela candelas", LIGHT, "1"),
    si("lm lumen lumens", LIGHT, "1"),
    si("lx lux", [-2, 0, 0, 0, 0, 0, 1, 0], "1"),
    // angles and plain numbers
    si("rad radian radians", NONE, "1"),
    unit("deg degree degrees", NONE, "pi/180"),
    unit("arcmin arcminute arcminutes", NONE, "pi/10800"),
    unit("arcsec arcsecond arcseconds", NONE, "pi/648000"),
    unit(
        "rev turn turns revolution revolutions cycle cycles",
        NONE,
        "2*pi",
    ),
    unit("grad gon", NONE, "pi/200"),
    unit("sr steradian steradians", NONE, "1"),
    unit("percent", NONE, "0.01"),
    unit("ppm", NONE, "1e-6"),
    unit("ppb", NONE, "1e-9"),
    // rates
    si("Hz hertz", RATE, "1"),
    unit("rpm", RATE, "2*pi/60"),
    si("Bq becquerel becquerels", RATE, "1"),
    unit("Ci curie curies", RATE, "3.7e10"),
    // speed and acceleration
    unit("mph", SPEED, "0.44704"),
    unit("kph", SPEED, "1000/3600"),
    unit("kn knot knots", SPEED, "1852/3600"),
    unit("c lightspeed speedoflight", SPEED, "299792458"),
    unit("gn gee", ACCELERATION, "9.80665"),
    // force, pressure
    si("N newton newtons", FORCE, "1"),
    unit("dyn dyne dynes", FORCE, "1e-5"),
    unit("lbf", FORCE, LBF),
    unit("kgf kp", FORCE, "9.80665"),
    si("Pa pascal pascals", PRESSURE, "1"),
    si("bar bars", PRESSURE, "1e5"),
    unit("atm atmosphere atmospheres", PRESSURE, "101325"),
    unit("torr Torr", PRESSURE, "101325/760"),
    unit("mmHg", PRESSURE, "133.322387415"),
    unit("inHg", PRESSURE, "25.4*133.322387415"),
    unit("psi", PRESSURE, "4.4482216152605/0.0254^2"),
    // energy, power
    si("J joule joules", ENERGY, "1"),
    si("cal calorie calories", ENERGY, "4.184"),
    si("eV electronvolt electronvolts", ENERGY, "1.602176634e-19"),
    si("Wh", ENERGY, "3600"),
    unit("BTU Btu btu", ENERGY, "1055.05585262"),
    unit("erg ergs", ENERGY, "1e-7"),
    unit("therm therms", ENERGY, "105505585.262"),
    unit("ftlbf", ENERGY, "0.3048*4.4482216152605"),
    unit("tonTNT", ENERGY, "4.184e9"),
    si("W watt watts", POWER, "1"),
    unit("hp horsepower", POWER, "550*0.3048*4.4482216152605"),
    unit("PS", POWER, "735.49875"),
    // electricity and magnetism
    si("C coulomb coulombs", CHARGE, "1"),
    si("Ah", CHARGE, "3600"),
    si("V volt volts", VOLTAGE, "1"),
    si("ohm ohms", RESISTANCE, "1"),
    si("F farad farads", CAPACITANCE, "1"),
    si("S siemens", [-2, -1, 3, 2, 0, 0, 0, 0], "1"),
    si("H henry henries", INDUCTANCE, "1"),
    si("Wb weber webers", FLUX, "1"),
    si("T tesla teslas", FLUX_DENSITY, "1"),
    unit("gauss", FLUX_DENSITY, "1e-4"),
    unit("Mx maxwell maxwells", FLUX, "1e-8"),
    // volume, area
    si("l L liter litre liters litres", VOLUME, "0.001"),
    unit("cc", VOLUME, "1e-6"),
    unit("gal gallon gallons", VOLUME, GALLON),
    unit("qt quart quarts", VOLUME, "0.003785411784/4"),
    unit("pt pint pints", VOLUME, "0.003785411784/8"),
    unit("cup cups", VOLUME, "0.003785411784/16"),
    unit("floz", VOLUME, "0.003785411784/128"),
    unit("tbsp", VOLUME, "0.003785411784/256"),
    unit("tsp", VOLUME, "0.003785411784/768"),
    unit("bbl barrel barrels", VOLUME, "42*0.003785411784"),
    unit("impgal", VOLUME, "0.00454609"),
    unit("ha hectare hectares", AREA, "10000"),
    unit("acre acres", AREA, "4046.8564224"),
    unit("barn barns", AREA, "1e-28"),
    // information
    si("bit bits", DATA, "1"),
    si("B byte bytes", DATA, "8"),
    si("bps", [0, 0, -1, 0, 0, 0, 0, 1], "1"),
    // radiation, chemistry, fluids
    si("Gy gray grays", DOSE, "1"),
    si("Sv sievert sieverts", DOSE, "1"),
    unit("rem", DOSE, "0.01"),
    si("kat katal katals", [0, 0, -1, 0, 0, 1, 0, 0], "1"),
    si("M molar", [-3, 0, 0, 0, 0, 1, 0, 0], "1000"),
    unit("poise", [-1, 1, -1, 0, 0, 0, 0, 0], "0.1"),
    unit("stokes", [2, 0, -1, 0, 0, 0, 0, 0], "1e-4"),
    // constants of nature: the ones that the SI fixes are exact, the measured ones are the
    // values of CODATA 2022
    unit(
        "planck hplanck",
        [2, 1, -1, 0, 0, 0, 0, 0],
        "6.62607015e-34",
    ),
    unit("hbar", [2, 1, -1, 0, 0, 0, 0, 0], "6.62607015e-34/(2*pi)"),
    unit(
        "k_B kboltz boltzmann",
        [2, 1, -2, 0, -1, 0, 0, 0],
        "1.380649e-23",
    ),
    unit(
        "N_A NA avogadro",
        [0, 0, 0, 0, 0, -1, 0, 0],
        "6.02214076e23",
    ),
    unit("qe elementarycharge", CHARGE, "1.602176634e-19"),
    unit(
        "Rgas gasconstant",
        [2, 1, -2, 0, -1, -1, 0, 0],
        "6.02214076e23*1.380649e-23",
    ),
    unit(
        "faraday",
        [0, 0, 1, 1, 0, -1, 0, 0],
        "6.02214076e23*1.602176634e-19",
    ),
    unit(
        "sigmaSB stefanboltzmann",
        [0, 1, -3, 0, -4, 0, 0, 0],
        "2*pi^5*1.380649e-23^4/(15*6.62607015e-34^3*299792458^2)",
    ),
    measured(
        "G gravitationalconstant",
        [3, -1, -2, 0, 0, 0, 0, 0],
        "6.67430e-11",
        6,
    ),
    measured("me electronmass", MASS, "9.1093837139e-31", 11),
    measured("mp protonmass", MASS, "1.67262192595e-27", 12),
    measured("mn neutronmass", MASS, "1.67492750056e-27", 12),
    measured("mu0", [1, 1, -2, -2, 0, 0, 0, 0], "1.25663706127e-6", 12),
    measured("eps0", [-3, -1, 4, 2, 0, 0, 0, 0], "8.8541878188e-12", 11),
    measured("alphafs finestructure", NONE, "7.2973525643e-3", 11),
    measured("a0 bohrradius", LENGTH, "5.29177210544e-11", 12),
    measured("Msun solarmass", MASS, "1.98847e30", 6),
    measured("Mearth earthmass", MASS, "5.9722e24", 5),
    unit("Rsun solarradius", LENGTH, "6.957e8"),
    measured("Rearth earthradius", LENGTH, "6.371e6", 4),
];

/// Prefixes with the power of ten (or of two) they stand for; the long names first.
const PREFIXES: &[(&str, &str)] = &[
    ("quetta", "1e30"),
    ("ronna", "1e27"),
    ("yotta", "1e24"),
    ("zetta", "1e21"),
    ("exa", "1e18"),
    ("peta", "1e15"),
    ("tera", "1e12"),
    ("giga", "1e9"),
    ("mega", "1e6"),
    ("kilo", "1e3"),
    ("hecto", "1e2"),
    ("deca", "1e1"),
    ("deka", "1e1"),
    ("deci", "1e-1"),
    ("centi", "1e-2"),
    ("milli", "1e-3"),
    ("micro", "1e-6"),
    ("nano", "1e-9"),
    ("pico", "1e-12"),
    ("femto", "1e-15"),
    ("atto", "1e-18"),
    ("zepto", "1e-21"),
    ("yocto", "1e-24"),
    ("ronto", "1e-27"),
    ("quecto", "1e-30"),
    ("kibi", "2^10"),
    ("mebi", "2^20"),
    ("gibi", "2^30"),
    ("tebi", "2^40"),
    ("pebi", "2^50"),
    ("exbi", "2^60"),
    ("Ki", "2^10"),
    ("Mi", "2^20"),
    ("Gi", "2^30"),
    ("Ti", "2^40"),
    ("Pi", "2^50"),
    ("Ei", "2^60"),
    ("da", "1e1"),
    ("Q", "1e30"),
    ("R", "1e27"),
    ("Y", "1e24"),
    ("Z", "1e21"),
    ("E", "1e18"),
    ("P", "1e15"),
    ("T", "1e12"),
    ("G", "1e9"),
    ("M", "1e6"),
    ("k", "1e3"),
    ("h", "1e2"),
    ("d", "1e-1"),
    ("c", "1e-2"),
    ("m", "1e-3"),
    ("u", "1e-6"),
    ("n", "1e-9"),
    ("p", "1e-12"),
    ("f", "1e-15"),
    ("a", "1e-18"),
    ("z", "1e-21"),
    ("y", "1e-24"),
    ("r", "1e-27"),
    ("q", "1e-30"),
];

fn exact(def: &Def, name: &str) -> bool {
    def.names.split(' ').any(|n| n == name)
}

thread_local! {
    /// The units that `newunit` made in this session, each with its size in SI base units.
    static OWN: RefCell<Vec<(&'static Def, Expr)>> = const { RefCell::new(Vec::new()) };
}

/// The unit of a name: a unit as it is, or a prefix before a unit that takes prefixes.
fn find(name: &str) -> Option<(&'static Def, Option<&'static str>)> {
    if let Some(def) = OWN.with(|own| own.borrow().iter().map(|(def, _)| *def).find(|d| exact(d, name))) {
        return Some((def, None));
    }
    if let Some(def) = UNITS.iter().find(|d| exact(d, name)) {
        return Some((def, None));
    }
    PREFIXES.iter().find_map(|(prefix, power)| {
        let rest = name.strip_prefix(prefix)?;
        let def = UNITS.iter().find(|d| d.prefixes && exact(d, rest))?;
        // (the powers of two belong to bits and bytes)
        let binary = power.starts_with('2');
        (!binary || def.dim == DATA).then_some((def, Some(*power)))
    })
}

/// Whether the parser reads `name` as a unit inside the unit commands.
pub fn is_unit(name: &str) -> bool {
    find(name).is_some()
}

/// A unit or a power of one, as written.
fn is_unit_factor(e: &Expr) -> bool {
    match e {
        Expr::Sym(name) => is_unit(name),
        Expr::Pow(base, _) => base.sym_name().is_some_and(is_unit),
        _ => false,
    }
}

/// A product as it is written, `5 km/h`, as the node `quantity(5, km/h)`: a zero keeps its unit
/// so. `None` for everything else.
pub fn written(e: &Expr) -> Option<Expr> {
    let Expr::Mul(factors) = e else {
        return None;
    };
    let (units, others): (Vec<Expr>, Vec<Expr>) = factors.iter().cloned().partition(is_unit_factor);
    let has_unit = |f: &Expr| f.any(&|x| x.sym_name().is_some_and(is_unit));
    if units.is_empty() || others.is_empty() || others.iter().any(has_unit) {
        return None;
    }
    let one = |mut v: Vec<Expr>| match v.len() {
        1 => v.pop().expect("one"),
        _ => Expr::Mul(v.into()),
    };
    Some(Expr::call("quantity", vec![one(others), one(units)]))
}

/// The commands whose arguments have units.
pub fn takes_units(command: &str) -> bool {
    matches!(command, "convert" | "siunits" | "dimension" | "quantity" | "newunit" | "removeunit")
}

thread_local! {
    static SIZES: RefCell<HashMap<&'static str, Expr>> = RefCell::new(HashMap::new());
}

/// The value of a size as it is written in the tables.
fn constant(text: &'static str) -> Expr {
    if let Some(hit) = SIZES.with(|s| s.borrow().get(text).cloned()) {
        return hit;
    }
    let raw = crate::parser::parse(text)
        .ok()
        .and_then(|mut p| p.stmts.pop())
        .expect("a size of the unit tables is an expression");
    let value = map_bottom_up(&raw, &mut |x| x);
    SIZES.with(|s| s.borrow_mut().insert(text, value.clone()));
    value
}

fn size_of(def: &Def) -> Expr {
    if let Some(size) = OWN.with(|own| own.borrow().iter().find(|(d, _)| std::ptr::eq(*d, def)).map(|(_, size)| size.clone())) {
        return size;
    }
    if def.digits == 0 {
        return constant(def.size);
    }
    // a measured value: a number with the digits that are known
    let value = parse_decimal(def.size).expect("a measured size is a decimal number");
    let prec = numeric::bits_for(def.digits) + 32;
    numeric::to_float_expr(Complex::with_val(prec, &value), def.digits)
}

fn zero_dim() -> Dim {
    std::array::from_fn(|_| Rat::new())
}

fn dim_of(def: &Def) -> Dim {
    std::array::from_fn(|k| Rat::from(def.dim[k]))
}

fn is_plain(dim: &Dim) -> bool {
    dim.iter().all(|p| p.is_zero())
}

/// A product of the names with these powers.
fn product_of(names: &[&str; 8], dim: &Dim) -> Expr {
    mul(names
        .iter()
        .zip(dim)
        .filter(|(_, p)| !p.is_zero())
        .map(|(n, p)| pow(Expr::sym(n), Expr::rational(p.clone())))
        .collect())
}

/// The kind of a quantity with these powers, in words.
fn kind(dim: &Dim) -> String {
    if is_plain(dim) {
        return "a plain number".to_string();
    }
    product_of(&KINDS, dim).to_string()
}

struct Reader {
    /// Names that were taken as numbers, not units.
    strangers: Vec<String>,
}

impl Reader {
    /// The size of `e` in SI base units and the powers of the base units.
    fn measure(&mut self, e: &Expr) -> Result<(Expr, Dim), String> {
        match e {
            Expr::Sym(name) => match find(name) {
                Some((def, power)) => {
                    let size = match power {
                        Some(p) => mul(vec![constant(p), size_of(def)]),
                        None => size_of(def),
                    };
                    let size = if def.zero.is_empty() {
                        size
                    } else {
                        // one degree of a scale by itself: the point 1 on it
                        mul(vec![add(vec![Expr::one(), constant(def.zero)]), size])
                    };
                    Ok((size, dim_of(def)))
                }
                None => {
                    if !crate::expr::is_const_name(name)
                        && !self.strangers.iter().any(|s| s == &**name)
                    {
                        self.strangers.push(name.to_string());
                    }
                    Ok((e.clone(), zero_dim()))
                }
            },
            Expr::Mul(factors) => {
                // a number of degrees of a scale is a point on it; in a product with other
                // units a degree is a step
                let degrees: Vec<&Expr> = factors.iter().filter(|f| is_scale(f)).collect();
                if let [degree] = degrees.as_slice() {
                    let others: Vec<Expr> =
                        factors.iter().filter(|f| !is_scale(f)).cloned().collect();
                    let (count, dim) = self.measure(&mul(others))?;
                    if is_plain(&dim) {
                        let (def, _) = find(degree.sym_name().expect("a symbol")).expect("a unit");
                        let point = add(vec![count, constant(def.zero)]);
                        return Ok((mul(vec![point, size_of(def)]), dim_of(def)));
                    }
                }
                let (mut size, mut dim) = (Vec::new(), zero_dim());
                for f in factors.iter() {
                    let (s, d) = self.step(f)?;
                    size.push(s);
                    for (total, part) in dim.iter_mut().zip(d) {
                        *total += part;
                    }
                }
                Ok((mul(size), dim))
            }
            Expr::Pow(base, k) => {
                let (size, dim) = self.step(base)?;
                if is_plain(&dim) {
                    let (power, power_dim) = self.measure(k)?;
                    if !is_plain(&power_dim) {
                        return Err(format!("the exponent {k} has a unit"));
                    }
                    return Ok((pow(size, power), dim));
                }
                let Some(r) = k.as_rat() else {
                    return Err(format!(
                        "a unit to the power {k}: the power must be a number"
                    ));
                };
                Ok((pow(size, (**k).clone()), dim.map(|p| p * r.clone())))
            }
            Expr::Add(terms) => {
                let mut sizes = Vec::new();
                let mut first: Option<Dim> = None;
                for t in terms.iter() {
                    let (s, d) = self.measure(t)?;
                    match &first {
                        Some(f) if *f != d => {
                            return Err(format!("{} and {} do not add up", kind(f), kind(&d)));
                        }
                        Some(_) => {}
                        None => first = Some(d),
                    }
                    sizes.push(s);
                }
                Ok((add(sizes), first.unwrap_or_else(zero_dim)))
            }
            Expr::Fn(name, args) if &**name == "quantity" && args.len() == 2 => {
                let (count, plain) = self.measure(&args[0])?;
                if !is_plain(&plain) {
                    return Err(format!("{} has a unit already", args[0]));
                }
                // a number of degrees of a scale is a point on it
                let degree = args[1].sym_name().and_then(find);
                if let Some((def, _)) = degree.filter(|(def, _)| !def.zero.is_empty()) {
                    let point = add(vec![count, constant(def.zero)]);
                    return Ok((mul(vec![point, size_of(def)]), dim_of(def)));
                }
                let (step, dim) = self.step(&args[1])?;
                Ok((mul(vec![count, step]), dim))
            }
            Expr::Fn(name, args) => {
                // a function takes plain numbers
                let mut values = Vec::new();
                for a in args.iter() {
                    let (s, d) = self.measure(a)?;
                    if !is_plain(&d) {
                        return Err(format!("{name} of {}: it takes a plain number", kind(&d)));
                    }
                    values.push(s);
                }
                Ok((func(name, values), zero_dim()))
            }
            Expr::List(_) | Expr::Rel(..) | Expr::Bool(_) => Err(format!("{e} is no quantity")),
            Expr::Num(_) | Expr::Float(_) => Ok((e.clone(), zero_dim())),
        }
    }

    /// The same with the degrees of a temperature scale as steps.
    fn step(&mut self, e: &Expr) -> Result<(Expr, Dim), String> {
        match e.sym_name().and_then(find) {
            Some((def, _)) if !def.zero.is_empty() => Ok((size_of(def), dim_of(def))),
            _ => self.measure(e),
        }
    }
}

/// A degree of a temperature scale whose zero is not absolute zero.
fn is_scale(e: &Expr) -> bool {
    e.sym_name()
        .and_then(find)
        .is_some_and(|(def, _)| !def.zero.is_empty())
}

fn quantity(value: Expr, unit: Expr) -> Expr {
    let value = simplify::simplify(&value);
    if unit.is_one() {
        return value;
    }
    Expr::call("quantity", vec![value, unit])
}

/// What a command found, with its remarks.
pub struct Answer {
    pub value: Expr,
    pub notes: Vec<String>,
}

fn answer(value: Expr, reader: Reader) -> Answer {
    let notes = reader
        .strangers
        .iter()
        .map(|name| format!("{name} is taken as a number, not as a unit"))
        .collect();
    Answer { value, notes }
}

/// `q` in the unit `target`; a list of units gives whole numbers of all but the last.
pub fn convert(q: &Expr, target: &Expr) -> Result<Answer, String> {
    let mut reader = Reader {
        strangers: Vec::new(),
    };
    let (size, dim) = reader.measure(q)?;
    let targets: Vec<Expr> = match target {
        Expr::List(items) if !items.is_empty() => items.to_vec(),
        Expr::List(_) => return Err("convert(quantity, unit) or convert(quantity, [units])".into()),
        one => vec![one.clone()],
    };
    let mut out = Vec::new();
    let mut left = size;
    for (k, unit) in targets.iter().enumerate() {
        let scale = is_scale(unit);
        let (step, unit_dim) = reader.step(unit)?;
        if unit_dim != dim {
            let strangers = match reader.strangers.as_slice() {
                [] => String::new(),
                [one] => format!("{one} is no unit: "),
                names => format!("{} are no units: ", names.join(", ")),
            };
            return Err(format!(
                "{strangers}{} is {}, {} is {}",
                q,
                kind(&dim),
                unit,
                kind(&unit_dim)
            ));
        }
        let mut count = div(left.clone(), step.clone());
        if scale && targets.len() == 1 {
            let (def, _) = find(unit.sym_name().expect("a symbol")).expect("a unit");
            count = sub(count, constant(def.zero));
        }
        let count = simplify::simplify(&count);
        if k + 1 < targets.len() {
            // whole numbers of the larger units
            let whole = numeric::real_value(&count, numeric::digits())
                .and_then(|v| v.floor().to_integer())
                .ok_or_else(|| format!("{q} has no value to split into units"))?;
            let whole = Expr::integer(whole);
            left = sub(left, mul(vec![whole.clone(), step]));
            out.push(quantity(whole, unit.clone()));
        } else {
            out.push(quantity(count, unit.clone()));
        }
    }
    let value = match out.len() {
        1 => out.pop().expect("one"),
        _ => Expr::list(out),
    };
    Ok(answer(value, reader))
}

/// `q` in SI base units.
pub fn siunits(q: &Expr) -> Result<Answer, String> {
    let mut reader = Reader {
        strangers: Vec::new(),
    };
    let (size, dim) = reader.measure(q)?;
    Ok(answer(quantity(size, product_of(&BASE, &dim)), reader))
}

/// The kind of `q`: a product of powers of length, mass, time and so on.
pub fn dimension(q: &Expr) -> Result<Answer, String> {
    let mut reader = Reader {
        strangers: Vec::new(),
    };
    let (_, dim) = reader.measure(q)?;
    Ok(answer(product_of(&KINDS, &dim), reader))
}

/// `newunit("name", quantity)`: a unit of the user's own, until the session ends or
/// `removeunit` takes it away.
pub fn define(name: &Expr, q: &Expr) -> Result<Answer, String> {
    let usage = "newunit(\"name\", quantity): a new unit by a quantity of units that are there, as newunit(\"fortnight\", 14 day)";
    let word = name.sym_name().or_else(|| crate::radix::text_of(name)).ok_or(usage)?.to_string();
    let mut signs = word.chars();
    if !signs.next().is_some_and(char::is_alphabetic) || !signs.all(|c| c.is_alphanumeric() || c == '_') {
        return Err(format!("newunit: the name of a unit begins with a letter and holds letters, digits and _, {word} does not"));
    }
    if find(&word).is_some() {
        return Err(format!("newunit: {word} is a unit already"));
    }
    if crate::names::function(&word).is_some() || crate::expr::is_const_name(&word) {
        return Err(format!("newunit: {word} is the name of a function or of a constant"));
    }
    let mut reader = Reader { strangers: Vec::new() };
    let (size, dim) = reader.measure(q)?;
    if let Some(stranger) = reader.strangers.first() {
        return Err(format!("newunit: {stranger} is no unit; a new unit is made of units that are there"));
    }
    let mut powers = [0i8; 8];
    for (power, own) in dim.iter().zip(powers.iter_mut()) {
        *own = (*power.denom() == 1).then(|| power.numer().to_i8()).flatten().ok_or("newunit: a unit has whole powers of the base units")?;
    }
    let size = simplify::simplify(&size);
    if size.is_zero() {
        return Err("newunit: a unit of the size zero measures nothing".into());
    }
    let def: &'static Def = Box::leak(Box::new(unit(Box::leak(word.clone().into_boxed_str()), powers, "")));
    OWN.with(|own| own.borrow_mut().push((def, size)));
    Ok(Answer { value: Expr::sym(&word), notes: Vec::new() })
}

/// `removeunit("name")`: a unit that `newunit` made is a name again.
pub fn forget(name: &Expr) -> Result<Answer, String> {
    let word = name.sym_name().or_else(|| crate::radix::text_of(name)).ok_or("removeunit(\"name\"): the name of a unit that newunit made")?.to_string();
    let found = OWN.with(|own| {
        let mut own = own.borrow_mut();
        let before = own.len();
        own.retain(|(def, _)| !exact(def, &word));
        own.len() < before
    });
    match found {
        true => Ok(Answer { value: Expr::Bool(true), notes: Vec::new() }),
        false if find(&word).is_some() => Err(format!("removeunit: {word} is a unit of the program itself; only the units that newunit made go away")),
        false => Err(format!("removeunit: {word} is no unit")),
    }
}

/// `quantity(value, unit)` as it is written.
pub fn make(value: &Expr, unit: &Expr) -> Result<Answer, String> {
    let mut reader = Reader {
        strangers: Vec::new(),
    };
    let (_, dim) = reader.measure(value)?;
    if !is_plain(&dim) {
        return Err(format!("quantity(value, unit): {value} has a unit already"));
    }
    reader.measure(unit)?;
    Ok(answer(quantity(value.clone(), unit.clone()), reader))
}

/// A result with quantities in it as one quantity: a multiple of a quantity keeps its unit,
/// everything else comes in SI base units. `None` when the units do not fit together.
pub fn settle(e: &Expr) -> Option<Expr> {
    let parts = |q: &Expr| q.fn_args("quantity").map(|a| (a[0].clone(), a[1].clone()));
    if let Expr::Mul(factors) = e {
        let found: Vec<(Expr, Expr)> = factors.iter().filter_map(parts).collect();
        let others: Vec<Expr> = factors
            .iter()
            .filter(|f| parts(f).is_none())
            .cloned()
            .collect();
        if let [(value, unit)] = found.as_slice()
            && others.iter().all(|f| !f.has_fn("quantity") && f.is_const())
        {
            let mut all = others;
            all.push(value.clone());
            return Some(quantity(mul(all), unit.clone()));
        }
    }
    if let Expr::Add(terms) = e {
        let found: Option<Vec<(Expr, Expr)>> = terms
            .iter()
            .map(|t| match settle(t) {
                Some(q) => parts(&q),
                None => parts(t),
            })
            .collect();
        if let Some(found) = found
            && let Some((_, unit)) = found.first()
            && found.iter().all(|(_, u)| u == unit)
        {
            let total = add(found.iter().map(|(v, _)| v.clone()).collect());
            return Some(quantity(total, unit.clone()));
        }
    }
    siunits(e).ok().map(|a| a.value)
}

/// Text of `--units`: every unit with its names and its size in SI base units.
pub fn help() -> String {
    let mut out = String::from(
        "Units (read inside convert, siunits, dimension and quantity; the size of one in SI base units):\n",
    );
    for def in UNITS {
        let names = def.names.replace(' ', ", ");
        let dim = dim_of(def);
        let base = product_of(&BASE, &dim);
        let size = size_of(def);
        let mut text = if !def.zero.is_empty() {
            format!(
                "a scale: absolute zero is at -{}, a degree is {size} K",
                def.zero
            )
        } else if base.is_one() {
            size.to_string()
        } else {
            Expr::call("quantity", vec![size, base]).to_string()
        };
        if def.digits > 0 {
            text.push_str(" (measured)");
        }
        if def.prefixes {
            text.push_str(" (takes prefixes)");
        }
        out.push_str(&format!("    {names:<44} {text}\n"));
    }
    out.push_str("Prefixes before the units that take them (km, ms, GHz, kWh, MeV, mmol, MiB):\n");
    let mut line = String::from("   ");
    for (prefix, power) in PREFIXES {
        let item = format!(" {prefix} = {}", power.replace("1e", "10^"));
        if line.len() + item.len() > 96 {
            out.push_str(&line);
            out.push('\n');
            line = String::from("   ");
        }
        line.push_str(&item);
        line.push(',');
    }
    out.push_str(line.trim_end_matches(','));
    out.push('\n');
    out
}

/// The value of a quantity as it is printed: a fraction that ends as a decimal number is
/// written as one.
pub fn shown(value: &Expr) -> String {
    if let Some(r) = value.as_rat()
        && !r.is_integer()
    {
        let mut rest = r.denom().clone();
        let (mut twos, mut fives) = (0u32, 0u32);
        while rest.is_divisible_u(2) {
            rest /= 2u32;
            twos += 1;
        }
        while rest.is_divisible_u(5) {
            rest /= 5u32;
            fives += 1;
        }
        let places = twos.max(fives);
        if rest == 1 && places <= 400 {
            let scaled = r.numer() * rug::Integer::from(10).pow(places) / r.denom().clone();
            let digits = scaled.clone().abs().to_string();
            let sign = if scaled < 0 { "-" } else { "" };
            let width = places as usize;
            let padded = format!("{digits:0>width$}", width = width + 1);
            if padded.len() <= 21 {
                let (whole, part) = padded.split_at(padded.len() - width);
                return format!("{sign}{whole}.{part}");
            }
            return format!(
                "{sign}{}",
                scientific(&digits, digits.len() as i64 - 1 - i64::from(places))
            );
        }
    }
    if let Some(n) = value.as_int() {
        // a long whole number that ends in zeros
        let digits = n.clone().abs().to_string();
        let short = digits.trim_end_matches('0');
        if digits.len() > 21 && short.len() <= 21 {
            let sign = if *n < 0 { "-" } else { "" };
            return format!("{sign}{}", scientific(&digits, digits.len() as i64 - 1));
        }
        return value.to_string();
    }
    let plain = matches!(value, Expr::Float(_) | Expr::Sym(_));
    if plain && !crate::funcs::is_neg_form(value) {
        value.to_string()
    } else {
        format!("({value})")
    }
}

/// The digits `d1 d2 d3 ...` as `d1.d2d3...e<exponent>`, without the zeros at the end.
fn scientific(digits: &str, exponent: i64) -> String {
    let digits = digits.trim_end_matches('0');
    let (first, rest) = digits.split_at(1);
    if rest.is_empty() {
        format!("{first}e{exponent}")
    } else {
        format!("{first}.{rest}e{exponent}")
    }
}

/// A unit as it is printed after a value: `s^-1` and not `1/s`.
pub fn unit_text(unit: &Expr) -> String {
    let (top, low) = crate::canon::numer_denom(unit);
    if !top.is_one() || low.is_one() {
        return unit.to_string();
    }
    let factors: Vec<Expr> = match &low {
        Expr::Mul(a) => a.to_vec(),
        other => vec![other.clone()],
    };
    let parts: Vec<String> = factors
        .iter()
        .map(|f| pow(f.clone(), Expr::minus_one()))
        .map(|f| match &f {
            Expr::Pow(base, k) => format!("{base}^{k}"),
            other => other.to_string(),
        })
        .collect();
    parts.join("*")
}

#[cfg(test)]
mod own_units {
    fn said(input: &str) -> String {
        crate::eval::run_line(input).map_or_else(|e| format!("error: {e}"), |lines| lines.join("|"))
    }

    #[test]
    fn a_unit_of_the_users_own_is_read_until_it_is_removed() {
        // (the units of a session stay: every line of this test is of one session)
        assert_eq!(said("newunit(\"smoot\", 1.7018 m); convert(364.4 smoot, m)"), "smoot|620.13592 m");
        assert_eq!(said("newunit(\"earsmoot\", smoot/67); convert(134 earsmoot, m)"), "earsmoot|3.4036 m");
        assert!(said("newunit(\"smoot\", 2 m)").contains("smoot is a unit already"));
        assert_eq!(said("newunit(\"warp\", 3 c); dimension(2 warp)"), "warp|length/time");
        assert!(said("removeunit(\"smoot\"); convert(1 smoot, m)").contains("smoot is no unit"));
        assert_eq!(said("removeunit(\"warp\")"), "true");
        assert!(said("newunit(\"km\", 2 m)").contains("is a unit already"));
        assert!(said("newunit(\"cos\", 2 m)").contains("name of a function"));
        assert!(said("newunit(\"zz\", 2 qq)").contains("qq is no unit"));
        assert!(said("removeunit(\"km\")").contains("of the program itself"));
    }
}
