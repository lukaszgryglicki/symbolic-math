//! The earth as a sphere and as an ellipsoid: distances, courses and places along great
//! circles, geodesics and lines of one course; areas; the coordinates of a place in space, on
//! a plane that touches the earth, and as seen from another place; latitudes of other kinds;
//! a few maps; angles and lengths in other units. Angles are degrees unless radians are said.

use crate::arrays::{whole, word};
use crate::canon::{div, mul};
use crate::expr::Expr;
use crate::numeric;
use crate::simplify::simplify;
use rug::ops::Pow;
use rug::{Complex, Float};
use std::cell::RefCell;

type Res = Result<Expr, String>;

fn bits() -> u32 {
    crate::mfun::bits()
}

fn int(x: i64) -> Float {
    Float::with_val(bits(), x)
}

fn pi() -> Float {
    Float::with_val(bits(), rug::float::Constant::Pi)
}

/// A decimal number of a table, as it is written.
fn dec(text: &str) -> Float {
    Float::with_val(bits(), Float::parse(text).expect("a number"))
}

fn real(e: &Expr, who: &str) -> Result<Float, String> {
    crate::mfun::real(e, bits(), who)
}

fn hypot(x: &Float, y: &Float) -> Float {
    x.clone().hypot(y)
}

fn fl<T>(x: T) -> Float
where
    Float: rug::Assign<T>,
{
    Float::with_val(bits(), x)
}

fn zero() -> Float {
    Float::new(bits())
}

fn half_pi() -> Float {
    pi() / 2u32
}

/// The size below which a number is nothing with the digits of the session.
fn dust() -> Float {
    fl(1) >> (bits() - 64)
}

fn at_pole(lat: &Float) -> bool {
    fl(lat.abs_ref()) >= half_pi() - dust()
}

/// An angle from 0 up to a full turn, which is 0 again.
fn turned(x: Float) -> Float {
    let turn = pi() * 2u32;
    let mut y = x % &turn;
    if y < 0 {
        y += &turn;
    }
    let near = dust() * 64u32;
    match y <= near || fl(&turn - &y) <= near {
        true => zero(),
        false => y,
    }
}

fn atan2(y: &Float, x: &Float) -> Float {
    y.clone().atan2(x)
}

fn sin_cos(x: &Float) -> (Float, Float) {
    x.clone().sin_cos(Float::new(bits()))
}

/// What is left of a value after the digits of the session: nothing when it is the dust of
/// a subtraction of numbers of the size `scale`.
fn settled(x: Float, scale: &Float) -> Float {
    let least = Float::with_val(bits(), scale.abs_ref()).max(&int(1))
        * Float::with_val(bits(), Float::i_pow_u(10, numeric::digits() + 6)).recip();
    match Float::with_val(bits(), x.abs_ref()) <= least {
        true => Float::new(bits()),
        false => x,
    }
}

fn shown(x: &Float) -> Expr {
    crate::mfun::value(x)
}

fn list(values: &[Float]) -> Expr {
    Expr::list(values.iter().map(shown).collect())
}

/// An angle between -pi and pi: pi itself stays.
fn wrapped(x: Float) -> Float {
    let turn = pi() * 2u32;
    let mut y = x % &turn;
    if y > pi() {
        y -= &turn;
    } else if y < -pi() {
        y += &turn;
    }
    y
}

/// An ellipsoid of revolution: its half axis at the equator and its flattening (0: a sphere).
#[derive(Clone, Debug)]
pub struct Ellipsoid {
    pub a: Float,
    pub f: Float,
}

/// The ellipsoids with names: the half axis in metres and the inverse of the flattening
/// (nothing for a sphere).
const NAMED: &[(&str, &str, &str)] = &[
    ("wgs84", "6378137", "298.257223563"),
    ("grs80", "6378137", "298.257222101"),
    ("grs67", "6378160", "298.247167427"),
    ("wgs72", "6378135", "298.26"),
    ("wgs66", "6378145", "298.25"),
    ("wgs60", "6378165", "298.3"),
    ("clarke66", "6378206.4", "294.9786982"),
    ("clarke80", "6378249.145", "293.465"),
    ("airy1830", "6377563.396", "299.3249646"),
    ("airy1849", "6377340.189", "299.3249646"),
    ("bessel", "6377397.155", "299.1528128"),
    ("everest", "6377276.345", "300.8017"),
    ("international", "6378388", "297"),
    ("hayford", "6378388", "297"),
    ("krasovsky", "6378245", "298.3"),
    ("iau65", "6378160", "298.25"),
    ("iau68", "6378160", "298.2472"),
    ("pz90", "6378136", "298.25784"),
    ("earth", "6378137", "298.257223563"),
    ("unitsphere", "1", ""),
    ("sun", "695700000", "111111"),
    ("moon", "1738100", "833.33"),
    ("mercury", "2440530", "1075"),
    ("venus", "6051800", ""),
    ("mars", "3396190", "169.894"),
    ("jupiter", "71492000", "15.4144"),
    ("saturn", "60268000", "10.208"),
    ("uranus", "25559000", "43.616"),
    ("neptune", "24764000", "58.5437"),
    ("pluto", "1188300", ""),
];

/// The bodies that are taken as spheres, with their radius in metres.
const SPHERES: &[(&str, &str)] = &[
    ("earth", "6371000"),
    ("moon", "1738000"),
    ("sun", "694460000"),
    ("mercury", "2439000"),
    ("venus", "6051000"),
    ("mars", "3390000"),
    ("jupiter", "69882000"),
    ("saturn", "58235000"),
    ("uranus", "25362000"),
    ("neptune", "24622000"),
    ("pluto", "1151000"),
    ("unitsphere", "1"),
    ("unit", "1"),
];

fn sphere_names() -> String {
    SPHERES
        .iter()
        .map(|(n, _)| *n)
        .collect::<Vec<_>>()
        .join(", ")
}

/// The metres of a unit of length.
const LENGTHS: &[(&str, &str)] = &[
    ("m", "1"),
    ("meter", "1"),
    ("meters", "1"),
    ("metre", "1"),
    ("metres", "1"),
    ("km", "1000"),
    ("kilometer", "1000"),
    ("kilometers", "1000"),
    ("kilometre", "1000"),
    ("kilometres", "1000"),
    ("cm", "1/100"),
    ("centimeter", "1/100"),
    ("centimeters", "1/100"),
    ("mm", "1/1000"),
    ("millimeter", "1/1000"),
    ("millimeters", "1/1000"),
    ("micron", "1/1000000"),
    ("microns", "1/1000000"),
    ("nm", "1852"),
    ("nmi", "1852"),
    ("nautmi", "1852"),
    ("nauticalmile", "1852"),
    ("nauticalmiles", "1852"),
    ("ft", "381/1250"),
    ("foot", "381/1250"),
    ("feet", "381/1250"),
    ("in", "127/5000"),
    ("inch", "127/5000"),
    ("inches", "127/5000"),
    ("yd", "1143/1250"),
    ("yds", "1143/1250"),
    ("yard", "1143/1250"),
    ("yards", "1143/1250"),
    ("mi", "201168/125"),
    ("mile", "201168/125"),
    ("miles", "201168/125"),
    ("internationalmile", "201168/125"),
    ("internationalmiles", "201168/125"),
    ("sm", "6336000/3937"),
    ("statutemile", "6336000/3937"),
    ("statutemiles", "6336000/3937"),
    ("surveymile", "6336000/3937"),
    ("surveymiles", "6336000/3937"),
    ("surveyfoot", "1200/3937"),
    ("surveyfeet", "1200/3937"),
    ("surveyft", "1200/3937"),
    ("sf", "1200/3937"),
];

/// The metres of a unit of length, as a fraction.
fn metres(unit: &str) -> Option<Expr> {
    let plain: String = unit
        .to_ascii_lowercase()
        .chars()
        .filter(|c| !matches!(c, ' ' | '.' | '-' | '_'))
        .collect();
    LENGTHS
        .iter()
        .find(|(name, _)| *name == plain)
        .map(|(_, size)| {
            let mut parts = size.split('/');
            let top = Expr::integer(
                parts
                    .next()
                    .unwrap_or("1")
                    .parse::<rug::Integer>()
                    .expect("a number"),
            );
            match parts.next() {
                Some(bottom) => crate::canon::div(
                    top,
                    Expr::integer(bottom.parse::<rug::Integer>().expect("a number")),
                ),
                None => top,
            }
        })
}

fn units_known() -> String {
    "m, km, cm, mm, micron, nm (nautical miles), ft, in, yd, mi (miles of 1609.344 m), sm (statute miles of 6336000/3937 m), sf (survey feet)".to_string()
}

impl Ellipsoid {
    fn sphere(radius: Float) -> Ellipsoid {
        Ellipsoid {
            a: radius,
            f: Float::new(bits()),
        }
    }

    /// The ellipsoid of a name, in metres.
    fn named(name: &str) -> Option<Ellipsoid> {
        let plain: String = name
            .to_ascii_lowercase()
            .chars()
            .filter(|c| !matches!(c, ' ' | '-' | '_'))
            .collect();
        NAMED
            .iter()
            .find(|(n, _, _)| *n == plain)
            .map(|(_, a, inverse)| Ellipsoid {
                a: dec(a),
                f: match inverse.is_empty() {
                    true => Float::new(bits()),
                    false => dec(inverse).recip(),
                },
            })
    }

    /// An ellipsoid as it is given: [half axis, eccentricity], a name, or the radius of a sphere.
    fn of(e: &Expr, who: &str) -> Result<Ellipsoid, String> {
        let wrong = || {
            format!(
                "{who}: an ellipsoid is [half axis, eccentricity], the radius of a sphere, or a name: {}; not {e}",
                NAMED
                    .iter()
                    .map(|(n, _, _)| *n)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        if let Some(name) = crate::radix::text_of(e) {
            return Ellipsoid::named(name).ok_or_else(wrong);
        }
        match e.as_list() {
            Some([a, ecc]) => {
                let (a, ecc) = (real(a, who)?, real(ecc, who)?);
                if a <= 0 || !(0..1).contains(&ecc) {
                    return Err(wrong());
                }
                // f = 1 - sqrt(1 - e^2)
                let f = int(1) - (int(1) - Float::with_val(bits(), ecc.square_ref())).sqrt();
                Ok(Ellipsoid { a, f })
            }
            Some(_) => Err(wrong()),
            None => {
                let radius = real(e, who).map_err(|_| wrong())?;
                match radius > 0 {
                    true => Ok(Ellipsoid::sphere(radius)),
                    false => Err(wrong()),
                }
            }
        }
    }

    fn e2(&self) -> Float {
        Float::with_val(bits(), &self.f * (int(2) - &self.f))
    }

    fn ecc(&self) -> Float {
        self.e2().sqrt()
    }

    fn b(&self) -> Float {
        Float::with_val(bits(), &self.a * (int(1) - &self.f))
    }

    fn round(&self) -> bool {
        self.f.is_zero()
    }

    /// [half axis, eccentricity], as MATLAB writes an ellipsoid.
    fn vector(&self) -> Expr {
        list(&[self.a.clone(), self.ecc()])
    }

    /// The radius across the meridian (of the circle that fits the surface from east to west).
    fn normal(&self, sin_lat: &Float) -> Float {
        let w = (int(1) - self.e2() * Float::with_val(bits(), sin_lat.square_ref())).sqrt();
        Float::with_val(bits(), &self.a / w)
    }

    /// The sine and the cosine of the reduced latitude of a latitude.
    fn reduced(&self, lat: &Float) -> (Float, Float) {
        let (s, c) = sin_cos(lat);
        let s = s * (int(1) - &self.f);
        let size = hypot(&s, &c);
        (s / &size, c / size)
    }

    /// The length of the meridian from the equator to a latitude.
    fn meridian(&self, lat: &Float) -> Float {
        if self.round() {
            return Float::with_val(bits(), &self.a * lat);
        }
        let e2 = self.e2();
        let scale = Float::with_val(bits(), &self.a * (int(1) - &e2));
        scale
            * integral(&Float::new(bits()), lat, &|t| {
                let s = t.clone().sin();
                let w = int(1) - Float::with_val(bits(), &e2 * s.square());
                Float::with_val(bits(), w.clone() * w.sqrt()).recip()
            })
    }

    /// The latitude whose meridian from the equator has a length.
    fn meridian_inverse(&self, length: &Float) -> Float {
        let e2 = self.e2();
        let mut lat = Float::with_val(bits(), length / &self.a);
        for _ in 0..200 {
            let s = lat.clone().sin();
            let w = int(1) - Float::with_val(bits(), &e2 * s.square());
            let slope = Float::with_val(bits(), &self.a * (int(1) - &e2))
                / Float::with_val(bits(), w.clone() * w.sqrt());
            let step = (self.meridian(&lat) - length) / slope;
            lat -= &step;
            if quiet(&step, &lat) {
                break;
            }
        }
        lat
    }

    /// The isometric latitude: the height on the map of Mercator.
    fn isometric(&self, lat: &Float) -> Float {
        if at_pole(lat) {
            return Float::with_val(bits(), rug::float::Special::Infinity) * lat.clone().signum();
        }
        let e = self.ecc();
        lat.clone().tan().asinh() - fl(&e * fl(&e * lat.clone().sin()).atanh())
    }

    fn isometric_inverse(&self, height: &Float) -> Float {
        if height.is_infinite() {
            return half_pi() * height.clone().signum();
        }
        let e = self.ecc();
        // tan(lat) = sinh(height + e atanh(e sin(lat)))
        let mut t = height.clone().sinh();
        for _ in 0..500 {
            let s = fl(&t / (int(1) + fl(t.square_ref())).sqrt());
            let next = fl(height + fl(&e * fl(&e * &s).atanh())).sinh();
            let step = fl(&next - &t);
            let size = next.clone();
            t = next;
            if quiet(&step, &size) {
                break;
            }
        }
        t.atan()
    }

    /// The conformal latitude: of the sphere on which angles stay.
    fn conformal(&self, lat: &Float) -> Float {
        match at_pole(lat) {
            true => lat.clone(),
            false => self.isometric(lat).sinh().atan(),
        }
    }

    fn conformal_inverse(&self, lat: &Float) -> Float {
        match at_pole(lat) {
            true => lat.clone(),
            false => self.isometric_inverse(&lat.clone().tan().asinh()),
        }
    }

    /// The rectifying latitude: of the sphere on which the lengths of the meridian stay.
    fn rectifying(&self, lat: &Float) -> Float {
        self.meridian(lat) / self.meridian(&half_pi()) * half_pi()
    }

    fn rectifying_inverse(&self, lat: &Float) -> Float {
        match at_pole(lat) {
            true => lat.clone(),
            false => self.meridian_inverse(&(fl(lat / half_pi()) * self.meridian(&half_pi()))),
        }
    }

    /// The radius of the meridian at a latitude.
    fn along(&self, sin_lat: &Float) -> Float {
        let e2 = self.e2();
        let w = int(1) - fl(&e2 * fl(sin_lat.square_ref()));
        fl(&self.a * (int(1) - &e2)) / fl(w.clone() * w.sqrt())
    }

    /// q of the areas: twice the sine of the latitude on a sphere.
    fn q(&self, sin_lat: &Float) -> Float {
        if self.round() {
            return fl(sin_lat * 2u32);
        }
        let e = self.ecc();
        let es = fl(&e * sin_lat);
        (int(1) - self.e2()) * (fl(sin_lat / (int(1) - fl(es.square_ref()))) + es.atanh() / e)
    }

    /// The sine of the latitude of a q.
    fn q_inverse(&self, q: &Float) -> Float {
        if self.round() {
            return fl(q / 2u32).min(&int(1)).max(&int(-1));
        }
        let e2 = self.e2();
        let mut s = fl(q / self.q(&int(1))).min(&int(1)).max(&int(-1));
        for _ in 0..200 {
            let w = int(1) - fl(&e2 * fl(s.square_ref()));
            let slope = (int(1) - &e2) * 2u32 / fl(w.square_ref());
            let step = (self.q(&s) - q) / slope;
            s -= &step;
            if quiet(&step, &int(1)) {
                break;
            }
        }
        s.min(&int(1)).max(&int(-1))
    }

    fn authalic_inverse(&self, lat: &Float) -> Float {
        match self.round() || at_pole(lat) {
            true => lat.clone(),
            false => self
                .q_inverse(&(self.q(&int(1)) * lat.clone().sin()))
                .asin(),
        }
    }

    /// The authalic latitude: of the sphere of the same area.
    fn authalic(&self, lat: &Float) -> Float {
        if self.round() || at_pole(lat) {
            return lat.clone();
        }
        let ratio = self.q(&lat.clone().sin()) / self.q(&int(1));
        ratio.min(&int(1)).max(&int(-1)).asin()
    }

    /// The radius of the sphere of the same area.
    fn authalic_radius(&self) -> Float {
        if self.round() {
            return self.a.clone();
        }
        let e = self.ecc();
        let e2 = self.e2();
        let half = (int(1) + (int(1) - &e2) * e.clone().atanh() / e) / 2u32;
        Float::with_val(bits(), &self.a * half.sqrt())
    }
}

/// Whether a step of an iteration is below the digits that are kept.
fn quiet(step: &Float, value: &Float) -> bool {
    match (step.get_exp(), value.get_exp()) {
        (None, _) => true,
        (Some(s), e) => i64::from(s) < i64::from(e.unwrap_or(0).max(0)) - i64::from(bits()) + 8,
    }
}

/// The integral of a smooth function of period pi from one angle to another, by the rule
/// of Gauss on pieces that are an eighth of a turn at most.
fn integral(from: &Float, to: &Float, f: &dyn Fn(&Float) -> Float) -> Float {
    let span = Float::with_val(bits(), to - from);
    if span.is_zero() {
        return span;
    }
    let pieces = (Float::with_val(bits(), span.abs_ref()) / (pi() / 8u32))
        .ceil()
        .to_f64()
        .clamp(1.0, 1e6) as u32;
    let width = Float::with_val(bits(), &span / pieces);
    let rule = crate::multivar::rule((bits() / 4 + 8) as usize, bits());
    let mut total = Float::new(bits());
    for piece in 0..pieces {
        let start = Float::with_val(bits(), from + Float::with_val(bits(), &width * piece));
        for (x, w) in rule.iter() {
            total += f(&Float::with_val(
                bits(),
                &start + Float::with_val(bits(), &width * x),
            )) * w;
        }
    }
    total * width
}

/// The geodesics of an ellipsoid: the shortest lines on it.
struct Geodesic<'a> {
    on: &'a Ellipsoid,
}

/// A geodesic through a place: the sines and cosines that describe it on the sphere that
/// goes with the ellipsoid.
struct Line {
    /// The sine and the cosine of the course where the line crosses the equator.
    s0: Float,
    c0: Float,
    /// k^2 of the integrals.
    k2: Float,
}

/// What is found between two places: the length of the shortest line, the courses at its
/// ends (that of the way onward at the second place), and the area between the line and
/// the equator (east of the first place counts as positive when the line runs east).
struct Found {
    length: Float,
    first: Float,
    second: Float,
    area: Float,
}

/// Along a meridian from a latitude, a length to the north or to the south: the latitude
/// there, whether that is on the meridian of the other side, and whether the way goes north.
fn meridian_walk(on: &Ellipsoid, lat: &Float, north: bool, length: &Float) -> (Float, bool, bool) {
    let quarter = on.meridian(&half_pi());
    let start = on.meridian(lat) / &quarter;
    let moved = fl(length / &quarter);
    let total = match north {
        true => start + moved,
        false => start - moved,
    };
    // quarters of the whole meridian, from -2 up to 2: beyond 1 is the other side
    let turns = (fl(&total + 2u32) / 4u32).floor();
    let t = total - turns * 4u32;
    let (part, other) = if t > 1 {
        (int(2) - t, true)
    } else if t < -1 {
        (int(-2) - t, true)
    } else {
        (t, false)
    };
    let there = match fl(part.abs_ref()) >= int(1) - dust() {
        true => half_pi() * part.signum(),
        false => on.meridian_inverse(&(part * quarter)),
    };
    (there, other, north != other)
}

impl Geodesic<'_> {
    fn line(
        &self,
        sin_course: &Float,
        cos_course: &Float,
        sin_reduced: &Float,
        cos_reduced: &Float,
    ) -> Line {
        let s0 = fl(sin_course * cos_reduced);
        let c0 = hypot(cos_course, &fl(sin_course * sin_reduced));
        let e2 = self.on.e2();
        let second = fl(&e2 / (int(1) - &e2));
        let k2 = second * fl(c0.square_ref());
        Line { s0, c0, k2 }
    }

    /// The length along a line between two arcs of the sphere, in units of the half axis
    /// at the pole.
    fn length(&self, line: &Line, from: &Float, to: &Float) -> Float {
        if self.on.round() {
            return fl(to - from);
        }
        integral(from, to, &|t| {
            let s = t.clone().sin();
            (int(1) + fl(&line.k2 * s.square())).sqrt()
        })
    }

    /// What the longitude on the ellipsoid lacks of that on the sphere between two arcs.
    fn lag(&self, line: &Line, from: &Float, to: &Float) -> Float {
        if self.on.round() {
            return zero();
        }
        let f = &self.on.f;
        let part = integral(from, to, &|t| {
            let s = t.clone().sin();
            let root = (int(1) + fl(&line.k2 * s.square())).sqrt();
            fl(int(2) - f) / (int(1) + fl(int(1) - f) * root)
        });
        fl(f * &line.s0) * part
    }

    /// The part of the area between a line and the equator that the sphere does not have,
    /// between two arcs.
    fn excess(&self, line: &Line, from: &Float, to: &Float) -> Float {
        if self.on.round() || line.s0.is_zero() || line.c0.is_zero() {
            return zero();
        }
        let e2 = self.on.e2();
        let second = fl(&e2 / (int(1) - &e2));
        // t(x) = x + sqrt(1/x + 1) asinh(sqrt(x))
        let t = |x: &Float| -> Float {
            match x.is_zero() {
                true => int(1),
                false => {
                    let root = x.clone().sqrt();
                    fl(x + (int(1) + x).sqrt() * root.clone().asinh() / root)
                }
            }
        };
        let top = t(&second);
        let part = integral(from, to, &|sigma| {
            let s = sigma.clone().sin();
            let x = fl(&line.k2 * fl(s.square_ref()));
            let below = fl(&second - &x);
            match below.is_zero() {
                true => zero(),
                false => -(fl(&top - t(&x)) / below) * s / 2u32,
            }
        });
        e2 * fl(self.on.a.square_ref()) * fl(&line.c0 * &line.s0) * part
    }

    /// From a place with a course, a length along the geodesic: the latitude there, the
    /// difference of the longitudes, and the course there. Radians.
    fn direct(&self, lat: &Float, course: &Float, length: &Float) -> (Float, Float, Float) {
        if at_pole(lat) {
            // from a pole the course is counted from the meridian of the longitude given
            let north_pole = *lat > 0;
            let (there, other, north) = meridian_walk(self.on, lat, !north_pole, length);
            let mut lon = match north_pole {
                true => pi() - course,
                false => course.clone(),
            };
            if other {
                lon += pi();
            }
            let onward = match north {
                true => zero(),
                false => pi(),
            };
            return (there, lon, onward);
        }
        let (sb, cb) = self.on.reduced(lat);
        let (sa, ca) = sin_cos(course);
        let line = self.line(&sa, &ca, &sb, &cb);
        let along = fl(&ca * &cb);
        let s1 = match sb.is_zero() && along.is_zero() {
            true => zero(),
            false => atan2(&sb, &along),
        };
        let target = fl(length / self.on.b());
        // the arc whose length from the first place is the target
        let mut s2 = fl(&s1 + &target);
        if !self.on.round() {
            for _ in 0..200 {
                let off = self.length(&line, &s1, &s2) - &target;
                let slope = (int(1) + fl(&line.k2 * s2.clone().sin().square())).sqrt();
                let step = off / slope;
                s2 -= &step;
                if quiet(&step, &s2) {
                    break;
                }
            }
        }
        let (ss1, cs1) = sin_cos(&s1);
        let (ss2, cs2) = sin_cos(&s2);
        let sb2 = fl(&line.c0 * &ss2);
        let cb2 = hypot(&line.s0, &fl(&line.c0 * &cs2));
        let course2 = atan2(&line.s0, &fl(&line.c0 * &cs2));
        let w1 = atan2(&fl(&line.s0 * &ss1), &cs1);
        let w2 = atan2(&fl(&line.s0 * &ss2), &cs2);
        let lon = fl(&w2 - &w1) - self.lag(&line, &s1, &s2);
        let lat2 = atan2(&sb2, &fl(&cb2 * (int(1) - &self.on.f)));
        (lat2, lon, course2)
    }

    /// Between two places: the shortest line. Radians; the length in the unit of the half
    /// axis. At a pole the courses are counted from the meridian of the longitude given.
    fn inverse(
        &self,
        lat1: &Float,
        lon1: &Float,
        lat2: &Float,
        lon2: &Float,
        with_area: bool,
    ) -> Found {
        let mut lon12 = wrapped(fl(lon2 - lon1));
        let mut east = if lon12 < 0 { -1 } else { 1 };
        lon12 *= east;
        let (mut near, mut far) = (lat1.clone(), lat2.clone());
        let swapped = fl(near.abs_ref()) < fl(far.abs_ref());
        if swapped {
            east = -east;
            std::mem::swap(&mut near, &mut far);
        }
        let north = if near < 0 { 1 } else { -1 };
        near *= north;
        far *= north;
        // now the first place is south of the equator or on it, the second one not farther
        // from the equator, and to the east by half a turn at most
        let (sb1, cb1) = self.on.reduced(&near);
        let (sb2, cb2) = self.on.reduced(&far);
        let b = self.on.b();
        let tiny = dust();
        let authalic = fl(self.on.authalic_radius().square());
        let (length, mut sa1, mut ca1, mut sa2, mut ca2, mut area);
        let pole = at_pole(&near);
        let over = fl(pi() - &lon12) <= tiny;
        if pole || lon12 <= tiny || over {
            // along a meridian: straight north, or over the nearer pole to the other side
            (sa1, ca1) = match (pole, over) {
                (true, _) => sin_cos(&lon12),
                (false, true) => (zero(), int(-1)),
                (false, false) => (zero(), int(1)),
            };
            (sa2, ca2) = (zero(), int(1));
            let (m1, m2) = (self.on.meridian(&near), self.on.meridian(&far));
            length = match over && !pole {
                true => self.on.meridian(&half_pi()) * 2u32 + &m1 + &m2,
                false => m2 - m1,
            };
            area = -fl(&authalic * &lon12);
        } else if sb1.is_zero() && sb2.is_zero() && lon12 <= fl(pi() * (int(1) - &self.on.f)) {
            // along the equator
            (sa1, ca1, sa2, ca2) = (int(1), zero(), int(1), zero());
            length = fl(&self.on.a * &lon12);
            area = zero();
        } else {
            // the course at the first place for which the line meets the second place
            let far_gap = fl(cb2.square_ref()) - fl(cb1.square_ref());
            let level = sb1.is_zero();
            let met = |course: &Float| -> (Float, Float, Float, Line, Float, Float) {
                let (sa, ca) = sin_cos(course);
                let line = self.line(&sa, &ca, &sb1, &cb1);
                // the cosine of the course at the second place, which is not past the turn
                let c2 = (fl(fl(&ca * &cb1).square()) + &far_gap).max(&zero()).sqrt() / &cb2;
                let s2 = fl(&line.s0 / &cb2);
                // from the equator the line goes south first: its arc starts half a turn back
                let back = level && ca < 0;
                let arc1 = match back {
                    true => -pi(),
                    false => atan2(&sb1, &fl(&ca * &cb1)),
                };
                let arc2 = atan2(&sb2, &fl(&c2 * &cb2));
                let (ss1, cs1) = sin_cos(&arc1);
                let (ss2, cs2) = sin_cos(&arc2);
                let w1 = match back {
                    true => -pi(),
                    false => atan2(&fl(&line.s0 * &ss1), &cs1),
                };
                let w2 = atan2(&fl(&line.s0 * &ss2), &cs2);
                let lon = fl(&w2 - &w1) - self.lag(&line, &arc1, &arc2);
                (lon, s2, c2, line, arc1, arc2)
            };
            // the longitude grows with the course: the course is found between 0 and pi
            // (two places on the equator: the way south of it, as short as the other one)
            let (mut low, mut high) = (zero(), pi());
            if level {
                low = half_pi();
            }
            let mut course = match self.on.round() {
                true => {
                    let (sl, cl) = sin_cos(&lon12);
                    atan2(&fl(&sl * &cb2), &(fl(&cb1 * &sb2) - fl(&sb1 * &cb2) * cl))
                }
                false => fl(&lon12 / 2u32).max(&tiny),
            };
            if course <= low || course >= high {
                course = fl(&low + &high) / 2u32;
            }
            let mut before: Option<(Float, Float)> = None;
            for _ in 0..1000 {
                let off = met(&course).0 - &lon12;
                if off.is_zero() {
                    break;
                }
                match off < 0 {
                    true => low = course.clone(),
                    false => high = course.clone(),
                }
                // the secant through the last two courses, or the middle when it leaves
                // what is known to hold the answer
                let middle = fl(&low + &high) / 2u32;
                let next = match &before {
                    Some((c0, f0)) if *f0 != off => {
                        let guess = fl(&course - fl(&off * fl(&course - c0)) / fl(&off - f0));
                        match guess > low && guess < high {
                            true => guess,
                            false => middle,
                        }
                    }
                    _ => middle,
                };
                let step = fl(&next - &course);
                before = Some((course, off));
                course = next;
                if quiet(&step, &int(1)) || fl(&high - &low) <= tiny {
                    break;
                }
            }
            let (_, s2, c2, line, arc1, arc2) = met(&course);
            (sa1, ca1) = sin_cos(&course);
            (sa2, ca2) = (s2, c2);
            length = b * self.length(&line, &arc1, &arc2);
            area = zero();
            if with_area {
                let turn = atan2(
                    &(fl(&sa2 * &ca1) - fl(&ca2 * &sa1)),
                    &(fl(&ca2 * &ca1) + fl(&sa2 * &sa1)),
                );
                area = authalic * turn + self.excess(&line, &arc1, &arc2);
            }
        }
        if swapped {
            std::mem::swap(&mut sa1, &mut sa2);
            std::mem::swap(&mut ca1, &mut ca2);
        }
        let (flip, way) = (if swapped { -1 } else { 1 }, east);
        sa1 *= flip * way;
        ca1 *= flip * north;
        sa2 *= flip * way;
        ca2 *= flip * north;
        area *= flip * way * north;
        if length.is_zero() {
            // no way: no course
            return Found {
                length,
                first: zero(),
                second: zero(),
                area: zero(),
            };
        }
        Found {
            length,
            first: atan2(&sa1, &ca1),
            second: atan2(&sa2, &ca2),
            area,
        }
    }
}

/// The great circle between two places of a sphere: the arc and the course at the first
/// place. Radians.
fn great_circle(lat1: &Float, lon1: &Float, lat2: &Float, lon2: &Float) -> (Float, Float) {
    let (s1, c1) = sin_cos(lat1);
    let (s2, c2) = sin_cos(lat2);
    let (sl, cl) = sin_cos(&fl(lon2 - lon1));
    let east = fl(&c2 * &sl);
    let north = fl(&c1 * &s2) - fl(&s1 * &c2) * &cl;
    let near = fl(&s1 * &s2) + fl(&c1 * &c2) * cl;
    let across = hypot(&east, &north);
    let course = match across <= dust() {
        true => zero(),
        false => atan2(&east, &north),
    };
    (atan2(&across, &near), course)
}

/// The line of one course between two places: its length and its course. Radians.
fn rhumb(on: &Ellipsoid, lat1: &Float, lon1: &Float, lat2: &Float, lon2: &Float) -> (Float, Float) {
    let east = wrapped(fl(lon2 - lon1));
    let rise = on.meridian(lat2) - on.meridian(lat1);
    let level = fl(rise.abs_ref()) <= fl(on.a.abs_ref()) * dust();
    if level {
        // along a parallel
        let (s, c) = sin_cos(lat1);
        let length = on.normal(&s) * c.max(&zero()) * fl(east.abs_ref());
        let course = match east.cmp0() {
            Some(std::cmp::Ordering::Less) => -half_pi(),
            Some(std::cmp::Ordering::Greater) => half_pi(),
            _ => zero(),
        };
        return (length, course);
    }
    let up = on.isometric(lat2) - on.isometric(lat1);
    let course = atan2(&east, &up);
    let length = fl(&rise / course.clone().cos());
    (length, course)
}

/// From a place with one course kept, a length: the place there. A line that runs into a
/// pole ends there.
fn rhumb_reckon(
    on: &Ellipsoid,
    lat: &Float,
    lon: &Float,
    length: &Float,
    course: &Float,
) -> (Float, Float) {
    let (sc, cc) = sin_cos(course);
    let rise = fl(length * &cc);
    let quarter = on.meridian(&half_pi());
    let height = on.meridian(lat) + &rise;
    if fl(height.abs_ref()) >= fl(&quarter - fl(on.a.abs_ref()) * dust()) {
        return (half_pi() * height.signum(), lon.clone());
    }
    let lat2 = on.meridian_inverse(&height);
    let level = fl(cc.abs_ref()) <= dust();
    let east = match level {
        true => {
            let (s, c) = sin_cos(lat);
            match c <= dust() {
                true => zero(),
                false => fl(length * &sc) / (on.normal(&s) * c),
            }
        }
        false => fl(&sc / &cc) * (on.isometric(&lat2) - on.isometric(lat)),
    };
    match east.is_finite() {
        true => (lat2, fl(lon + east)),
        false => (lat2, lon.clone()),
    }
}

/// The way between two places: the line of one course, or the shortest line.
fn way(
    on: &Ellipsoid,
    rhumb_line: bool,
    lat1: &Float,
    lon1: &Float,
    lat2: &Float,
    lon2: &Float,
    with_area: bool,
) -> Found {
    if rhumb_line {
        let (length, course) = rhumb(on, lat1, lon1, lat2, lon2);
        return Found {
            length,
            first: course.clone(),
            second: course,
            area: zero(),
        };
    }
    if on.round() && !with_area && !at_pole(lat1) && !at_pole(lat2) {
        let (arc, first) = great_circle(lat1, lon1, lat2, lon2);
        let second = match arc <= dust() {
            true => zero(),
            false => wrapped(great_circle(lat2, lon2, lat1, lon1).1 + pi()),
        };
        return Found {
            length: arc * &on.a,
            first,
            second,
            area: zero(),
        };
    }
    Geodesic { on }.inverse(lat1, lon1, lat2, lon2, with_area)
}

/// From a place, a length along a course: the place there and the course there.
fn onward(
    on: &Ellipsoid,
    rhumb_line: bool,
    lat: &Float,
    lon: &Float,
    length: &Float,
    course: &Float,
) -> (Float, Float, Float) {
    if rhumb_line {
        let (lat2, lon2) = rhumb_reckon(on, lat, lon, length, course);
        return (lat2, lon2, course.clone());
    }
    let (lat2, east, course2) = Geodesic { on }.direct(lat, course, length);
    (lat2, fl(lon + east), course2)
}

/// Whether a side of a polygon goes over the meridian of Greenwich: 1 to the east, -1 to
/// the west.
fn transit(lon1: &Float, lon2: &Float) -> i32 {
    let within = |x: Float| -> Float {
        let y = wrapped(x);
        match y <= -pi() {
            true => y + pi() * 2u32,
            false => y,
        }
    };
    let across = within(fl(lon2 - lon1));
    let (l1, l2) = (within(lon1.clone()), within(lon2.clone()));
    if across > 0 && ((l1 < 0 && l2 >= 0) || (l1 > 0 && l2.is_zero())) {
        1
    } else if across < 0 && l1 >= 0 && l2 < 0 {
        -1
    } else {
        0
    }
}

/// The area inside a polygon whose sides are shortest lines (the part of the surface that
/// is not more than half of it), and the length of its sides.
fn polygon(on: &Ellipsoid, lats: &[Float], lons: &[Float]) -> (Float, Float) {
    let whole = fl(on.authalic_radius().square()) * pi() * 4u32;
    let (mut sum, mut around, mut crossings) = (zero(), zero(), 0);
    for k in 0..lats.len() {
        let next = (k + 1) % lats.len();
        let found = Geodesic { on }.inverse(&lats[k], &lons[k], &lats[next], &lons[next], true);
        sum += found.area;
        around += found.length;
        crossings += transit(&lons[k], &lons[next]);
    }
    let mut area = fl(&sum - fl(&sum / &whole).round() * &whole);
    if crossings % 2 != 0 {
        let half = fl(&whole / 2u32);
        match area < 0 {
            true => area += half,
            false => area -= half,
        }
    }
    (settled(area.abs(), &whole), around)
}

type Vector = [Float; 3];

fn unit_vector(lat: &Float, lon: &Float) -> Vector {
    let (s, c) = sin_cos(lat);
    let (sl, cl) = sin_cos(lon);
    [fl(&c * &cl), c * sl, s]
}

/// The direction of a course at a place of the sphere.
fn heading(lat: &Float, lon: &Float, course: &Float) -> Vector {
    let (s, c) = sin_cos(lat);
    let (sl, cl) = sin_cos(lon);
    let (sa, ca) = sin_cos(course);
    [
        -fl(&ca * &s) * &cl - fl(&sa * &sl),
        -fl(&ca * &s) * &sl + fl(&sa * &cl),
        ca * c,
    ]
}

fn cross(a: &Vector, b: &Vector) -> Vector {
    [
        fl(&a[1] * &b[2]) - fl(&a[2] * &b[1]),
        fl(&a[2] * &b[0]) - fl(&a[0] * &b[2]),
        fl(&a[0] * &b[1]) - fl(&a[1] * &b[0]),
    ]
}

fn dot(a: &Vector, b: &Vector) -> Float {
    fl(&a[0] * &b[0]) + fl(&a[1] * &b[1]) + fl(&a[2] * &b[2])
}

fn norm(a: &Vector) -> Float {
    dot(a, a).sqrt()
}

/// The latitude and the longitude of a direction from the middle of the sphere.
fn place(v: &Vector) -> (Float, Float) {
    let level = hypot(&v[0], &v[1]);
    let lon = match level <= norm(v) * dust() {
        true => zero(),
        false => atan2(&v[1], &v[0]),
    };
    (atan2(&v[2], &level), lon)
}

// ------------------------------------------------------------------ what is given

/// What a command is told after its values.
#[derive(Default)]
struct Said {
    on: Option<Ellipsoid>,
    radians: bool,
    rhumb: Option<bool>,
    count: Option<usize>,
}

/// How many values a command takes, whether a count of places may follow, and whether the
/// kind of the way may be said.
struct Takes {
    least: usize,
    most: usize,
    counted: bool,
    tracks: bool,
}

fn takes(least: usize, most: usize) -> Takes {
    Takes {
        least,
        most,
        counted: false,
        tracks: false,
    }
}

/// A word of a command: a text, or a bare name that is no number.
fn plain_word(e: &Expr) -> Option<String> {
    if crate::radix::text_of(e).is_none() && numeric::real_value(e, 20).is_some() {
        return None;
    }
    word(e).map(|w| {
        w.chars()
            .filter(|c| !matches!(c, ' ' | '-' | '_'))
            .collect()
    })
}

fn track_word(w: &str) -> Option<bool> {
    match w {
        "gc" | "great" | "greatcircle" | "geodesic" => Some(false),
        "rh" | "rhumb" | "rhumbline" | "loxodrome" => Some(true),
        _ => None,
    }
}

fn unit_word(w: &str) -> Option<bool> {
    match w {
        "degrees" | "degree" | "deg" => Some(false),
        "radians" | "radian" | "rad" => Some(true),
        _ => None,
    }
}

fn ellipsoid_names() -> String {
    NAMED
        .iter()
        .map(|(n, _, _)| *n)
        .collect::<Vec<_>>()
        .join(", ")
}

fn empty(e: &Expr) -> bool {
    e.as_list().is_some_and(|l| l.is_empty())
}

/// The values of a call and what is said around them: the kind of the way first or
/// later, then an ellipsoid, the unit of the angles and a count of places.
fn said_of<'a>(a: &'a [Expr], who: &str, takes: Takes) -> Result<(Vec<&'a Expr>, Said), String> {
    let mut out = Said::default();
    let mut rest = a;
    if let Some(w) = rest.first().and_then(plain_word) {
        if let Some(kind) = track_word(&w).filter(|_| takes.tracks) {
            out.rhumb = Some(kind);
            rest = &rest[1..];
        } else if let Some(named) = Ellipsoid::named(&w) {
            out.on = Some(named);
            rest = &rest[1..];
        }
    }
    let mut values = Vec::new();
    while values.len() < takes.most
        && let Some(e) = rest.first()
        && plain_word(e).is_none()
    {
        values.push(e);
        rest = &rest[1..];
    }
    if values.len() < takes.least {
        return Err(forms(who));
    }
    let mut unit_said = false;
    for e in rest {
        match plain_word(e) {
            Some(w) => {
                if let Some(kind) = track_word(&w).filter(|_| takes.tracks) {
                    out.rhumb = Some(kind);
                } else if let Some(radians) = unit_word(&w) {
                    out.radians = radians;
                    unit_said = true;
                } else if let Some(named) = Ellipsoid::named(&w).filter(|_| out.on.is_none()) {
                    out.on = Some(named);
                } else {
                    let ways = match takes.tracks {
                        true => ", \"gc\" or \"rh\"",
                        false => "",
                    };
                    return Err(format!(
                        "{who}: {e} is not a word of this command: \"degrees\" or \"radians\"{ways}, or the name of an ellipsoid: {}",
                        ellipsoid_names()
                    ));
                }
            }
            None if empty(e) && !unit_said => {}
            None if out.on.is_none() && (!takes.counted || !unit_said || e.as_list().is_some()) => {
                out.on = Some(Ellipsoid::of(e, who)?);
            }
            None if takes.counted && out.count.is_none() => {
                out.count = Some(whole(e, "the number of places", 2, 100_000)? as usize);
            }
            None => return Err(forms(who)),
        }
    }
    Ok((values, out))
}

/// The ground of a calculation: the body, whether lengths are arcs of the unit sphere in
/// the unit of the angles, the unit of the angles, and the kind of the ways.
struct Frame {
    on: Ellipsoid,
    arcs: bool,
    radians: bool,
    rhumb: bool,
}

impl Frame {
    /// The frame of what was said; without a body the usual one of the command, or the
    /// unit sphere whose lengths are arcs.
    fn of(said: &Said, usual: Option<&str>) -> Frame {
        let given = said.on.clone().or_else(|| usual.and_then(Ellipsoid::named));
        Frame {
            arcs: given.is_none(),
            on: given.unwrap_or_else(|| Ellipsoid::sphere(int(1))),
            radians: said.radians,
            rhumb: said.rhumb.unwrap_or(false),
        }
    }

    fn angle_in(&self, x: &Float) -> Float {
        match self.radians {
            true => x.clone(),
            false => fl(x * pi()) / 180u32,
        }
    }

    fn angle_out(&self, x: Float) -> Float {
        match self.radians {
            true => settled(x, &int(1)),
            false => settled(x * 180u32 / pi(), &int(1)),
        }
    }

    /// A longitude: from half a turn to the west (without it) to half a turn to the east.
    fn lon_out(&self, x: Float) -> Float {
        let y = wrapped(x);
        match y <= dust() * 64u32 - pi() {
            true => self.angle_out(y + pi() * 2u32),
            false => self.angle_out(y),
        }
    }

    /// A course: from north over east, less than a full turn.
    fn course_out(&self, x: Float) -> Float {
        self.angle_out(turned(x))
    }

    fn latitude(&self, x: &Float, who: &str) -> Result<Float, String> {
        let lat = self.angle_in(x);
        let most = half_pi();
        if fl(lat.abs_ref()) > fl(&most + dust() * 64u32) {
            let (unit, limit) = match self.radians {
                true => ("radians", "pi/2"),
                false => ("degrees", "90"),
            };
            return Err(format!(
                "{who}: a latitude is from -{limit} to {limit} {unit}, not {}",
                shown(x)
            ));
        }
        Ok(lat.min(&most).max(&fl(-&most)))
    }

    fn length_in(&self, x: &Float) -> Float {
        match self.arcs {
            true => self.angle_in(x),
            false => x.clone(),
        }
    }

    fn length_out(&self, x: Float) -> Float {
        match self.arcs {
            true => self.angle_out(x),
            false => settled(x, &self.on.a),
        }
    }
}

fn leaves<'a>(e: &'a Expr, out: &mut Vec<&'a Expr>) {
    match e.as_list() {
        Some(items) => items.iter().for_each(|x| leaves(x, out)),
        None => out.push(e),
    }
}

fn alike(a: &Expr, b: &Expr) -> bool {
    match (a.as_list(), b.as_list()) {
        (Some(x), Some(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| alike(p, q)),
        (None, None) => true,
        _ => false,
    }
}

fn rebuilt(shape: &Expr, values: &mut dyn Iterator<Item = Expr>) -> Expr {
    match shape.as_list() {
        Some(items) => Expr::list(items.iter().map(|t| rebuilt(t, values)).collect()),
        None => values.next().expect("a value for every place"),
    }
}

/// The numbers of a list, or the one number.
fn reals(e: &Expr, who: &str) -> Result<Vec<Float>, String> {
    let mut all = Vec::new();
    leaves(e, &mut all);
    all.iter().map(|x| real(x, who)).collect()
}

/// What a command works out for the numbers of one place.
type Each<'a> = &'a dyn Fn(&[Float]) -> Result<Vec<Float>, String>;

/// A function of numbers at every place of the lists that are given (a number goes with
/// every place): its answers, each in the shape of the lists.
fn each(values: &[&Expr], who: &str, outs: usize, f: Each) -> Res {
    let shape = values.iter().find(|e| e.as_list().is_some()).copied();
    if let Some(shape) = shape
        && values
            .iter()
            .any(|e| e.as_list().is_some() && !alike(e, shape))
    {
        return Err(format!("{who}: the lists must have the same shape"));
    }
    let columns = values
        .iter()
        .map(|e| reals(e, who))
        .collect::<Result<Vec<_>, _>>()?;
    let size = columns.iter().map(Vec::len).max().unwrap_or(1);
    let size = match shape {
        Some(_) => size.min(
            columns
                .iter()
                .zip(values)
                .filter(|(_, e)| e.as_list().is_some())
                .map(|(c, _)| c.len())
                .min()
                .unwrap_or(0),
        ),
        None => 1,
    };
    let mut answers: Vec<Vec<Float>> = vec![Vec::with_capacity(size); outs];
    for k in 0..size {
        let row: Vec<Float> = columns
            .iter()
            .map(|c| c[k.min(c.len() - 1)].clone())
            .collect();
        for (slot, x) in answers.iter_mut().zip(f(&row)?) {
            slot.push(x);
        }
    }
    let mut made = answers.into_iter().map(|column| match shape {
        Some(shape) => rebuilt(shape, &mut column.iter().map(shown)),
        None => shown(&column[0]),
    });
    Ok(match outs {
        1 => made.next().expect("one answer"),
        _ => Expr::list(made.collect()),
    })
}

/// Two lists of the same length: the latitudes and the longitudes of places, in radians.
fn places(
    frame: &Frame,
    lats: &Expr,
    lons: &Expr,
    who: &str,
    least: usize,
) -> Result<(Vec<Float>, Vec<Float>), String> {
    let (lats, lons) = (reals(lats, who)?, reals(lons, who)?);
    if lats.len() != lons.len() || lats.len() < least {
        return Err(format!(
            "{who}: as many longitudes as latitudes, {least} at least; not {} and {}",
            lons.len(),
            lats.len()
        ));
    }
    let lats = lats
        .iter()
        .map(|x| frame.latitude(x, who))
        .collect::<Result<Vec<_>, _>>()?;
    let lons = lons.iter().map(|x| frame.angle_in(x)).collect();
    Ok((lats, lons))
}

/// The latitudes and the longitudes of places as they are shown: [[latitudes], [longitudes]].
fn track(frame: &Frame, lats: Vec<Float>, lons: Vec<Float>) -> Expr {
    let lats: Vec<Float> = lats.into_iter().map(|x| frame.angle_out(x)).collect();
    let lons: Vec<Float> = lons.into_iter().map(|x| frame.lon_out(x)).collect();
    Expr::list(vec![list(&lats), list(&lons)])
}

fn one(e: &Expr, who: &str) -> Result<Float, String> {
    match e.as_list() {
        Some(_) => Err(format!(
            "{who}: one number is needed here, not the list {e}"
        )),
        None => real(e, who),
    }
}

/// The places at `count` equal steps of a way from a place, from one length to another.
#[allow(clippy::too_many_arguments)]
fn stepped(
    frame: &Frame,
    lat: &Float,
    lon: &Float,
    course: &Float,
    from: &Float,
    to: &Float,
    count: usize,
) -> Expr {
    let (mut lats, mut lons) = (Vec::with_capacity(count), Vec::with_capacity(count));
    for k in 0..count {
        let share = fl(k as u32) / (count.max(2) - 1) as u32;
        let length = fl(from + fl(to - from) * share);
        let (lat2, lon2, _) = onward(&frame.on, frame.rhumb, lat, lon, &length, course);
        lats.push(lat2);
        lons.push(lon2);
    }
    track(frame, lats, lons)
}

/// The two places where two small circles of the unit sphere meet.
fn circles_meet(
    c1: &Vector,
    r1: &Float,
    c2: &Vector,
    r2: &Float,
    who: &str,
) -> Result<[Vector; 2], String> {
    let d = dot(c1, c2);
    let rest = int(1) - fl(d.square_ref());
    if rest <= dust() {
        return Err(format!(
            "{who}: the two circles have the same middle line: they meet nowhere or everywhere"
        ));
    }
    let (k1, k2) = (r1.clone().cos(), r2.clone().cos());
    let a = fl(&k1 - fl(&d * &k2)) / &rest;
    let b = fl(&k2 - fl(&d * &k1)) / &rest;
    let left = int(1) - fl(&a * &k1) - fl(&b * &k2);
    if left < -dust() * 64u32 {
        return Err(format!("{who}: the two circles do not meet"));
    }
    let t = (left.max(&zero()) / rest).sqrt();
    let across = cross(c1, c2);
    let make = |sign: i32| -> Vector {
        [0, 1, 2].map(|k| fl(&a * &c1[k]) + fl(&b * &c2[k]) + fl(&t * &across[k]) * sign)
    };
    Ok([make(1), make(-1)])
}

/// The commands of ways on the sphere and on an ellipsoid.
fn navigation(name: &str, a: &[Expr]) -> Result<Option<Expr>, String> {
    let tracks = |least: usize, most: usize, counted: bool| Takes {
        least,
        most,
        counted,
        tracks: true,
    };
    Ok(Some(match name {
        "distance" | "azimuth" => {
            let (v, said) = said_of(a, name, tracks(4, 4, false))?;
            let frame = Frame::of(&said, None);
            each(&v, name, 1, &|x| {
                let (lat1, lon1) = (frame.latitude(&x[0], name)?, frame.angle_in(&x[1]));
                let (lat2, lon2) = (frame.latitude(&x[2], name)?, frame.angle_in(&x[3]));
                let found = way(&frame.on, frame.rhumb, &lat1, &lon1, &lat2, &lon2, false);
                Ok(vec![match name {
                    "distance" => frame.length_out(found.length),
                    _ => frame.course_out(found.first),
                }])
            })?
        }
        "haversine" => {
            let (v, said) = said_of(a, name, takes(4, 4))?;
            let mut frame = Frame::of(&said, None);
            if frame.arcs {
                frame.on = Ellipsoid::sphere(int(6371));
                frame.arcs = false;
            }
            if !frame.on.round() {
                return Err(format!(
                    "{name}: the formula is for a sphere: give its radius (6371 km unless given)"
                ));
            }
            each(&v, name, 1, &|x| {
                let (lat1, lon1) = (frame.latitude(&x[0], name)?, frame.angle_in(&x[1]));
                let (lat2, lon2) = (frame.latitude(&x[2], name)?, frame.angle_in(&x[3]));
                Ok(vec![frame.length_out(
                    great_circle(&lat1, &lon1, &lat2, &lon2).0 * &frame.on.a,
                )])
            })?
        }
        "reckon" => {
            let (v, said) = said_of(a, name, tracks(4, 4, false))?;
            let frame = Frame::of(&said, None);
            each(&v, name, 2, &|x| {
                let (lat, lon) = (frame.latitude(&x[0], name)?, frame.angle_in(&x[1]));
                let (length, course) = (frame.length_in(&x[2]), frame.angle_in(&x[3]));
                let (lat2, lon2, _) = onward(&frame.on, frame.rhumb, &lat, &lon, &length, &course);
                Ok(vec![frame.angle_out(lat2), frame.lon_out(lon2)])
            })?
        }
        "geoddistance" => {
            let (v, said) = said_of(a, name, takes(4, 4))?;
            let frame = Frame::of(&said, Some("wgs84"));
            each(&v, name, 3, &|x| {
                let (lat1, lon1) = (frame.latitude(&x[0], name)?, frame.angle_in(&x[1]));
                let (lat2, lon2) = (frame.latitude(&x[2], name)?, frame.angle_in(&x[3]));
                let found = Geodesic { on: &frame.on }.inverse(&lat1, &lon1, &lat2, &lon2, false);
                Ok(vec![
                    frame.length_out(found.length),
                    frame.course_out(found.first),
                    frame.course_out(found.second),
                ])
            })?
        }
        "geodreckon" => {
            let (v, said) = said_of(a, name, takes(4, 4))?;
            let frame = Frame::of(&said, Some("wgs84"));
            each(&v, name, 3, &|x| {
                let (lat, lon) = (frame.latitude(&x[0], name)?, frame.angle_in(&x[1]));
                let (length, course) = (x[2].clone(), frame.angle_in(&x[3]));
                let (lat2, lon2, course2) = onward(&frame.on, false, &lat, &lon, &length, &course);
                Ok(vec![
                    frame.angle_out(lat2),
                    frame.lon_out(lon2),
                    frame.course_out(course2),
                ])
            })?
        }
        "geodarea" | "areaint" => {
            let (v, said) = said_of(a, name, takes(2, 2))?;
            let frame = Frame::of(
                &said,
                match name {
                    "geodarea" => Some("wgs84"),
                    _ => None,
                },
            );
            let (lats, lons) = places(&frame, v[0], v[1], name, 3)?;
            let (area, around) = polygon(&frame.on, &lats, &lons);
            match name {
                "geodarea" => list(&[area, settled(around, &frame.on.a)]),
                _ if frame.arcs => shown(&settled(area / (pi() * 4u32), &int(1))),
                _ => shown(&area),
            }
        }
        "areaquad" => {
            let (v, said) = said_of(a, name, takes(4, 4))?;
            let frame = Frame::of(&said, None);
            let radius = frame.on.authalic_radius();
            each(&v, name, 1, &|x| {
                let (lat1, lon1) = (frame.latitude(&x[0], name)?, frame.angle_in(&x[1]));
                let (lat2, lon2) = (frame.latitude(&x[2], name)?, frame.angle_in(&x[3]));
                let (s1, s2) = (
                    frame.on.authalic(&lat1).sin(),
                    frame.on.authalic(&lat2).sin(),
                );
                let part = fl(&lon2 - &lon1).abs() * fl(&s2 - &s1).abs();
                Ok(vec![match frame.arcs {
                    true => settled(part / (pi() * 4u32), &int(1)),
                    false => settled(part * fl(radius.square_ref()), &fl(radius.square_ref())),
                }])
            })?
        }
        "departure" => {
            let (v, said) = said_of(a, name, takes(3, 3))?;
            let frame = Frame::of(&said, None);
            each(&v, name, 1, &|x| {
                let (lon1, lon2) = (frame.angle_in(&x[0]), frame.angle_in(&x[1]));
                let lat = frame.latitude(&x[2], name)?;
                let (s, c) = sin_cos(&lat);
                let along = frame.on.normal(&s) * c.max(&zero()) * fl(&lon2 - &lon1).abs();
                Ok(vec![frame.length_out(along)])
            })?
        }
        "antipode" => {
            let (v, said) = said_of(a, name, takes(2, 2))?;
            let frame = Frame::of(&said, None);
            each(&v, name, 2, &|x| {
                let (lat, lon) = (frame.latitude(&x[0], name)?, frame.angle_in(&x[1]));
                Ok(vec![frame.angle_out(-lat), frame.lon_out(lon + pi())])
            })?
        }
        "meanm" => {
            let (v, said) = said_of(a, name, takes(2, 2))?;
            let frame = Frame::of(&said, None);
            let (lats, lons) = places(&frame, v[0], v[1], name, 1)?;
            let mut sum = [zero(), zero(), zero()];
            for (lat, lon) in lats.iter().zip(&lons) {
                let v = unit_vector(&frame.on.authalic(lat), lon);
                for k in 0..3 {
                    sum[k] += &v[k];
                }
            }
            if norm(&sum) <= dust() * 64u32 * lats.len() as u32 {
                return Err(format!(
                    "{name}: these places have no middle: their directions from the middle of the sphere cancel"
                ));
            }
            let (lat, lon) = place(&sum);
            list(&[
                frame.angle_out(frame.on.authalic_inverse(&lat)),
                frame.lon_out(lon),
            ])
        }
        "track2" => {
            let (v, said) = said_of(a, name, tracks(4, 4, true))?;
            let frame = Frame::of(&said, None);
            let (lat1, lon1) = (
                frame.latitude(&one(v[0], name)?, name)?,
                frame.angle_in(&one(v[1], name)?),
            );
            let (lat2, lon2) = (
                frame.latitude(&one(v[2], name)?, name)?,
                frame.angle_in(&one(v[3], name)?),
            );
            let found = way(&frame.on, frame.rhumb, &lat1, &lon1, &lat2, &lon2, false);
            stepped(
                &frame,
                &lat1,
                &lon1,
                &found.first,
                &zero(),
                &found.length,
                said.count.unwrap_or(100),
            )
        }
        "gcwaypts" => {
            let (v, said) = said_of(a, name, takes(4, 5))?;
            if said.on.is_some() {
                return Err(forms(name));
            }
            let frame = Frame::of(&said, None);
            let legs = match v.get(4) {
                Some(e) => whole(e, "the number of legs", 1, 100_000)? as usize,
                None => 10,
            };
            let (lat1, lon1) = (
                frame.latitude(&one(v[0], name)?, name)?,
                frame.angle_in(&one(v[1], name)?),
            );
            let (lat2, lon2) = (
                frame.latitude(&one(v[2], name)?, name)?,
                frame.angle_in(&one(v[3], name)?),
            );
            let found = way(&frame.on, false, &lat1, &lon1, &lat2, &lon2, false);
            stepped(
                &frame,
                &lat1,
                &lon1,
                &found.first,
                &zero(),
                &found.length,
                legs + 1,
            )
        }
        "track1" => {
            let (v, said) = said_of(a, name, tracks(3, 4, true))?;
            let frame = Frame::of(&said, None);
            let (lat, lon) = (
                frame.latitude(&one(v[0], name)?, name)?,
                frame.angle_in(&one(v[1], name)?),
            );
            let course = frame.angle_in(&one(v[2], name)?);
            let (from, to) = match v.get(3).filter(|e| !empty(e)) {
                Some(e) => match reals(e, name)?.as_slice() {
                    [to] => (zero(), frame.length_in(to)),
                    [from, to] => (frame.length_in(from), frame.length_in(to)),
                    _ => return Err(forms(name)),
                },
                None if frame.rhumb => {
                    return Err(format!(
                        "{name}: a line of one course needs its length: {}",
                        forms(name)
                    ));
                }
                None => (zero(), fl(&frame.on.a * pi()) * 2u32),
            };
            stepped(
                &frame,
                &lat,
                &lon,
                &course,
                &from,
                &to,
                said.count.unwrap_or(100),
            )
        }
        "scircle1" | "scircle2" => {
            let first = match name {
                "scircle1" => 3,
                _ => 4,
            };
            let (v, said) = said_of(a, name, tracks(first, first + 1, true))?;
            let frame = Frame::of(&said, None);
            let (lat, lon) = (
                frame.latitude(&one(v[0], name)?, name)?,
                frame.angle_in(&one(v[1], name)?),
            );
            let radius = match name {
                "scircle1" => frame.length_in(&one(v[2], name)?),
                _ => {
                    let (lat2, lon2) = (
                        frame.latitude(&one(v[2], name)?, name)?,
                        frame.angle_in(&one(v[3], name)?),
                    );
                    way(&frame.on, frame.rhumb, &lat, &lon, &lat2, &lon2, false).length
                }
            };
            let (from, to) = match v.get(first).filter(|e| !empty(e)) {
                Some(e) => match reals(e, name)?.as_slice() {
                    [to] => (zero(), frame.angle_in(to)),
                    [from, to] => (frame.angle_in(from), frame.angle_in(to)),
                    _ => return Err(forms(name)),
                },
                None => (zero(), pi() * 2u32),
            };
            let count = said.count.unwrap_or(100);
            let (mut lats, mut lons) = (Vec::with_capacity(count), Vec::with_capacity(count));
            for k in 0..count {
                let course = fl(&from + fl(&to - &from) * fl(k as u32) / (count - 1) as u32);
                let (lat2, lon2, _) = onward(&frame.on, frame.rhumb, &lat, &lon, &radius, &course);
                lats.push(lat2);
                lons.push(lon2);
            }
            track(&frame, lats, lons)
        }
        "legs" => {
            let (v, said) = said_of(a, name, tracks(2, 2, false))?;
            if said.on.is_some() {
                return Err(forms(name));
            }
            let frame = Frame::of(&said, None);
            let rhumb_line = said.rhumb.unwrap_or(true);
            let (lats, lons) = places(&frame, v[0], v[1], name, 2)?;
            // nautical miles of the sphere of 6371 km
            let miles = dec("6371000") / dec("1852");
            let (mut courses, mut lengths) = (Vec::new(), Vec::new());
            for k in 1..lats.len() {
                let found = way(
                    &frame.on,
                    rhumb_line,
                    &lats[k - 1],
                    &lons[k - 1],
                    &lats[k],
                    &lons[k],
                    false,
                );
                courses.push(frame.course_out(found.first));
                lengths.push(settled(found.length * &miles, &miles));
            }
            Expr::list(vec![list(&courses), list(&lengths)])
        }
        "gc2sc" => {
            let (v, said) = said_of(a, name, takes(3, 3))?;
            if said.on.is_some() {
                return Err(forms(name));
            }
            let frame = Frame::of(&said, None);
            each(&v, name, 3, &|x| {
                let (lat, lon) = (frame.latitude(&x[0], name)?, frame.angle_in(&x[1]));
                let course = frame.angle_in(&x[2]) + half_pi();
                let (lat2, lon2, _) = onward(&frame.on, false, &lat, &lon, &half_pi(), &course);
                Ok(vec![
                    frame.angle_out(lat2),
                    frame.lon_out(lon2),
                    frame.angle_out(half_pi()),
                ])
            })?
        }
        "gcxgc" | "scxsc" | "gcxsc" => {
            let (v, said) = said_of(a, name, takes(6, 6))?;
            if said.on.is_some() {
                return Err(forms(name));
            }
            let frame = Frame::of(&said, None);
            let x = v
                .iter()
                .map(|e| one(e, name))
                .collect::<Result<Vec<_>, _>>()?;
            let (lat1, lon1) = (frame.latitude(&x[0], name)?, frame.angle_in(&x[1]));
            let (lat2, lon2) = (frame.latitude(&x[3], name)?, frame.angle_in(&x[4]));
            // a great circle is the small circle of a quarter turn around its pole
            let pole = |lat: &Float, lon: &Float, course: &Float| {
                cross(&unit_vector(lat, lon), &heading(lat, lon, course))
            };
            let (c1, r1) = match name {
                "scxsc" => (unit_vector(&lat1, &lon1), frame.angle_in(&x[2])),
                _ => (pole(&lat1, &lon1, &frame.angle_in(&x[2])), half_pi()),
            };
            let (c2, r2) = match name {
                "gcxgc" => (pole(&lat2, &lon2, &frame.angle_in(&x[5])), half_pi()),
                _ => (unit_vector(&lat2, &lon2), frame.angle_in(&x[5])),
            };
            let met = circles_meet(&c1, &r1, &c2, &r2, name)?;
            let (p, q) = (place(&met[0]), place(&met[1]));
            Expr::list(vec![
                list(&[frame.angle_out(p.0), frame.angle_out(q.0)]),
                list(&[frame.lon_out(p.1), frame.lon_out(q.1)]),
            ])
        }
        "rhxrh" => {
            let (v, said) = said_of(a, name, takes(6, 6))?;
            if said.on.is_some() {
                return Err(forms(name));
            }
            let frame = Frame::of(&said, None);
            let x = v
                .iter()
                .map(|e| one(e, name))
                .collect::<Result<Vec<_>, _>>()?;
            let (lat1, lon1) = (frame.latitude(&x[0], name)?, frame.angle_in(&x[1]));
            let (lat2, lon2) = (frame.latitude(&x[3], name)?, frame.angle_in(&x[4]));
            if at_pole(&lat1) || at_pole(&lat2) {
                return Err(format!(
                    "{name}: a line of one course has no course at a pole"
                ));
            }
            // on the map of Mercator the lines are straight:
            // sin(c) (height - height0) - cos(c) (lon - lon0) = 0
            let (s1, c1) = sin_cos(&frame.angle_in(&x[2]));
            let (s2, c2) = sin_cos(&frame.angle_in(&x[5]));
            let (h1, h2) = (frame.on.isometric(&lat1), frame.on.isometric(&lat2));
            let lon2 = fl(&lon1 + wrapped(fl(&lon2 - &lon1)));
            let (k1, k2) = (
                fl(&s1 * &h1) - fl(&c1 * &lon1),
                fl(&s2 * &h2) - fl(&c2 * &lon2),
            );
            let det = fl(&c1 * &s2) - fl(&s1 * &c2);
            if fl(det.abs_ref()) <= dust() * 64u32 {
                return Err(format!(
                    "{name}: the two lines have the same course: they meet nowhere or everywhere"
                ));
            }
            let height = (fl(&c1 * &k2) - fl(&c2 * &k1)) / &det;
            let lon = (fl(&s1 * &k2) - fl(&s2 * &k1)) / det;
            list(&[
                frame.angle_out(frame.on.isometric_inverse(&height)),
                frame.lon_out(lon),
            ])
        }
        "interpm" => {
            let (v, said) = said_of(a, name, tracks(3, 3, false))?;
            if said.on.is_some() {
                return Err(forms(name));
            }
            let frame = Frame::of(&said, None);
            let straight = said.rhumb.is_none();
            let (lats, lons) = places(&frame, v[0], v[1], name, 1)?;
            let most = frame.angle_in(&one(v[2], name)?);
            if most <= 0 {
                return Err(format!("{name}: the largest step must be more than 0"));
            }
            let (mut out_lats, mut out_lons) = (vec![lats[0].clone()], vec![lons[0].clone()]);
            for k in 1..lats.len() {
                let (rise, east) = (fl(&lats[k] - &lats[k - 1]), fl(&lons[k] - &lons[k - 1]));
                let parts = (fl(rise.abs_ref()).max(&fl(east.abs_ref())) / &most - dust() * 64u32)
                    .ceil()
                    .to_f64()
                    .clamp(1.0, 1e6) as u32;
                let found = match straight {
                    true => None,
                    false => Some(way(
                        &frame.on,
                        frame.rhumb,
                        &lats[k - 1],
                        &lons[k - 1],
                        &lats[k],
                        &lons[k],
                        false,
                    )),
                };
                for part in 1..parts {
                    let share = fl(part) / parts;
                    let (lat, lon) = match &found {
                        None => (
                            fl(&lats[k - 1] + fl(&rise * &share)),
                            fl(&lons[k - 1] + fl(&east * &share)),
                        ),
                        Some(found) => {
                            let length = fl(&found.length * &share);
                            let (lat, lon, _) = onward(
                                &frame.on,
                                frame.rhumb,
                                &lats[k - 1],
                                &lons[k - 1],
                                &length,
                                &found.first,
                            );
                            (lat, lon)
                        }
                    };
                    out_lats.push(lat);
                    out_lons.push(lon);
                }
                out_lats.push(lats[k].clone());
                out_lons.push(lons[k].clone());
            }
            let lats: Vec<Float> = out_lats.into_iter().map(|x| frame.angle_out(x)).collect();
            let lons: Vec<Float> = match straight {
                true => out_lons.into_iter().map(|x| frame.angle_out(x)).collect(),
                false => out_lons.into_iter().map(|x| frame.lon_out(x)).collect(),
            };
            Expr::list(vec![list(&lats), list(&lons)])
        }
        _ => return Ok(None),
    }))
}

// ------------------------------------------------------------------ places in space

/// The place in space of a latitude, a longitude and a height above the ellipsoid.
fn ecef(on: &Ellipsoid, lat: &Float, lon: &Float, h: &Float) -> Vector {
    let (s, c) = sin_cos(lat);
    let c = c.max(&zero());
    let (sl, cl) = sin_cos(lon);
    let n = on.normal(&s);
    let level = fl(&n + h) * &c;
    [
        fl(&level * &cl),
        fl(&level * &sl),
        (n * (int(1) - on.e2()) + h) * s,
    ]
}

/// The latitude, the longitude and the height of a place in space.
fn geodetic(on: &Ellipsoid, v: &Vector) -> (Float, Float, Float) {
    let p = hypot(&v[0], &v[1]);
    let z = &v[2];
    let (e2, b) = (on.e2(), on.b());
    if p <= fl(&on.a * dust()) {
        // on the axis
        let lat = match z.cmp0() {
            Some(std::cmp::Ordering::Less) => -half_pi(),
            _ => half_pi(),
        };
        return (lat, zero(), fl(z.abs_ref()) - b);
    }
    let lon = atan2(&v[1], &v[0]);
    let second = fl(&e2 / (int(1) - &e2));
    // Bowring: the reduced latitude gives the latitude, which gives the reduced one
    let mut reduced = atan2(&fl(z * &on.a), &fl(&p * &b));
    let mut lat = zero();
    for _ in 0..500 {
        let (sb, cb) = sin_cos(&reduced);
        lat = atan2(
            &(fl(&second * &b) * fl(sb.square_ref()) * &sb + z),
            &(fl(&p - fl(&e2 * &on.a) * fl(cb.square_ref()) * &cb)),
        );
        let (s, c) = sin_cos(&lat);
        let next = atan2(&(s * (int(1) - &on.f)), &c);
        let step = fl(&next - &reduced);
        reduced = next;
        if quiet(&step, &int(1)) {
            break;
        }
    }
    let (s, c) = sin_cos(&lat);
    let h = fl(&p * &c) + fl(z * &s) - fl(&on.a * (int(1) - e2 * fl(s.square_ref())).sqrt());
    (lat, lon, h)
}

/// A step in space as east, north and up at a place.
fn enu_of(lat0: &Float, lon0: &Float, d: &Vector) -> Vector {
    let (s, c) = sin_cos(lat0);
    let (sl, cl) = sin_cos(lon0);
    let t = fl(&cl * &d[0]) + fl(&sl * &d[1]);
    [
        fl(&cl * &d[1]) - fl(&sl * &d[0]),
        fl(&c * &d[2]) - fl(&s * &t),
        fl(&c * &t) + fl(&s * &d[2]),
    ]
}

/// East, north and up at a place as a step in space.
fn enu_from(lat0: &Float, lon0: &Float, v: &Vector) -> Vector {
    let (s, c) = sin_cos(lat0);
    let (sl, cl) = sin_cos(lon0);
    let t = fl(&c * &v[2]) - fl(&s * &v[1]);
    [
        fl(&cl * &t) - fl(&sl * &v[0]),
        fl(&sl * &t) + fl(&cl * &v[0]),
        fl(&s * &v[2]) + fl(&c * &v[1]),
    ]
}

#[derive(Clone, Copy, PartialEq)]
enum Space {
    Geodetic,
    Ecef,
    Enu,
    Ned,
    Aer,
}

impl Space {
    fn named(word: &str) -> Option<Space> {
        Some(match word {
            "geodetic" => Space::Geodetic,
            "ecef" => Space::Ecef,
            "enu" => Space::Enu,
            "ned" => Space::Ned,
            "aer" => Space::Aer,
            _ => return None,
        })
    }

    fn local(self) -> bool {
        matches!(self, Space::Enu | Space::Ned | Space::Aer)
    }
}

/// East, north and up of three numbers of a local kind.
fn local_in(frame: &Frame, kind: Space, x: &[Float]) -> Vector {
    match kind {
        Space::Ned => [x[1].clone(), x[0].clone(), fl(-&x[2])],
        Space::Aer => {
            let (sa, ca) = sin_cos(&frame.angle_in(&x[0]));
            let (se, ce) = sin_cos(&frame.angle_in(&x[1]));
            let level = fl(&x[2] * &ce);
            [fl(&level * &sa), level * ca, fl(&x[2] * &se)]
        }
        _ => [x[0].clone(), x[1].clone(), x[2].clone()],
    }
}

fn local_out(frame: &Frame, kind: Space, v: Vector) -> Vec<Float> {
    let scale = norm(&v).max(&frame.on.a);
    let tidy = |x: Float| settled(x, &scale);
    let [e, n, u] = v;
    match kind {
        Space::Ned => vec![tidy(n), tidy(e), tidy(-u)],
        Space::Aer => {
            let level = hypot(&e, &n);
            let range = hypot(&level, &u);
            let course = match level <= fl(&range * dust()) {
                true => zero(),
                false => atan2(&e, &n),
            };
            vec![
                frame.course_out(course),
                frame.angle_out(atan2(&u, &level)),
                tidy(range),
            ]
        }
        _ => vec![tidy(e), tidy(n), tidy(u)],
    }
}

/// The three numbers of a place of one kind as those of another kind, with the place
/// from which the local kinds are seen.
fn moved(
    frame: &Frame,
    from: Space,
    to: Space,
    x: &[Float],
    who: &str,
) -> Result<Vec<Float>, String> {
    let origin = match x.len() >= 6 {
        true => Some((
            frame.latitude(&x[3], who)?,
            frame.angle_in(&x[4]),
            x[5].clone(),
        )),
        false => None,
    };
    let here = || {
        let (lat0, lon0, h0) = origin.as_ref().expect("a place to look from");
        ecef(&frame.on, lat0, lon0, h0)
    };
    let global: Option<Vector> = match from {
        Space::Geodetic => Some(ecef(
            &frame.on,
            &frame.latitude(&x[0], who)?,
            &frame.angle_in(&x[1]),
            &x[2],
        )),
        Space::Ecef => Some([x[0].clone(), x[1].clone(), x[2].clone()]),
        _ => None,
    };
    let local: Option<Vector> = match from.local() {
        true => Some(local_in(frame, from, x)),
        false => None,
    };
    Ok(match (global, local, to.local()) {
        (Some(p), _, true) => {
            let (lat0, lon0, _) = origin.as_ref().expect("a place to look from");
            let o = here();
            let d = [fl(&p[0] - &o[0]), fl(&p[1] - &o[1]), fl(&p[2] - &o[2])];
            local_out(frame, to, enu_of(lat0, lon0, &d))
        }
        (None, Some(v), true) => local_out(frame, to, v),
        (None, None, true) => unreachable!("a place is of one kind"),
        (given, v, false) => {
            let p = match (given, v) {
                (Some(p), _) => p,
                (None, Some(v)) => {
                    let (lat0, lon0, _) = origin.as_ref().expect("a place to look from");
                    let (o, d) = (here(), enu_from(lat0, lon0, &v));
                    [fl(&o[0] + &d[0]), fl(&o[1] + &d[1]), fl(&o[2] + &d[2])]
                }
                (None, None) => unreachable!("a place is of one kind"),
            };
            match to {
                Space::Geodetic => {
                    let (lat, lon, h) = geodetic(&frame.on, &p);
                    vec![
                        frame.angle_out(lat),
                        frame.lon_out(lon),
                        settled(h, &frame.on.a),
                    ]
                }
                _ => {
                    let scale = norm(&p).max(&frame.on.a);
                    p.into_iter().map(|x| settled(x, &scale)).collect()
                }
            }
        }
    })
}

/// The commands of places in space.
fn solid(name: &str, a: &[Expr]) -> Result<Option<Expr>, String> {
    match name {
        "elevation" => {
            let (v, said) = said_of(a, name, takes(6, 6))?;
            let frame = Frame::of(&said, Some("grs80"));
            return each(&v, name, 3, &|x| {
                let seen = [
                    x[3].clone(),
                    x[4].clone(),
                    x[5].clone(),
                    x[0].clone(),
                    x[1].clone(),
                    x[2].clone(),
                ];
                let found = moved(&frame, Space::Geodetic, Space::Aer, &seen, name)?;
                Ok(vec![found[1].clone(), found[2].clone(), found[0].clone()])
            })
            .map(Some);
        }
        "ecefoffset" => {
            let (v, said) = said_of(a, name, takes(6, 6))?;
            let frame = Frame::of(&said, Some("wgs84"));
            return each(&v, name, 3, &|x| {
                let p = ecef(
                    &frame.on,
                    &frame.latitude(&x[0], name)?,
                    &frame.angle_in(&x[1]),
                    &x[2],
                );
                let q = ecef(
                    &frame.on,
                    &frame.latitude(&x[3], name)?,
                    &frame.angle_in(&x[4]),
                    &x[5],
                );
                Ok((0..3)
                    .map(|k| settled(fl(&q[k] - &p[k]), &frame.on.a))
                    .collect())
            })
            .map(Some);
        }
        "lla2ecef" | "ecef2lla" => {
            let (v, said) = said_of(a, name, takes(1, 1))?;
            let frame = Frame::of(&said, Some("wgs84"));
            let rows: Vec<&Expr> = match v[0].as_list() {
                Some(items) if items.iter().all(|r| r.as_list().is_some()) => {
                    items.iter().collect()
                }
                Some(_) => vec![v[0]],
                None => return Err(forms(name)),
            };
            let mut out = Vec::new();
            for row in &rows {
                let x = reals(row, name)?;
                if x.len() != 3 {
                    return Err(forms(name));
                }
                let (from, to) = match name {
                    "lla2ecef" => (Space::Geodetic, Space::Ecef),
                    _ => (Space::Ecef, Space::Geodetic),
                };
                out.push(list(&moved(&frame, from, to, &x, name)?));
            }
            return Ok(Some(
                match v[0]
                    .as_list()
                    .is_some_and(|items| items.iter().all(|r| r.as_list().is_some()))
                {
                    true => Expr::list(out),
                    false => out.remove(0),
                },
            ));
        }
        _ => {}
    }
    let (base, vectors) = match name {
        "ecef2enuv" | "enu2ecefv" | "ecef2nedv" | "ned2ecefv" => (&name[..name.len() - 1], true),
        _ => (name, false),
    };
    let Some((from, to)) = base.split_once('2') else {
        return Ok(None);
    };
    let (Some(from), Some(to)) = (Space::named(from), Space::named(to)) else {
        return Ok(None);
    };
    if from == to {
        return Ok(None);
    }
    let count = match (vectors, from.local() == to.local()) {
        (true, _) => 5,
        (false, true) => 3,
        (false, false) => 6,
    };
    let (v, said) = said_of(a, name, takes(count, count))?;
    let frame = Frame::of(&said, Some("wgs84"));
    each(&v, name, 3, &|x| {
        if vectors {
            let (lat0, lon0) = (frame.latitude(&x[3], name)?, frame.angle_in(&x[4]));
            let scale = int(1);
            let turned = match from {
                Space::Ecef => {
                    let v = enu_of(&lat0, &lon0, &[x[0].clone(), x[1].clone(), x[2].clone()]);
                    match to {
                        Space::Ned => [v[1].clone(), v[0].clone(), fl(-&v[2])],
                        _ => v,
                    }
                }
                _ => enu_from(&lat0, &lon0, &local_in(&frame, from, x)),
            };
            let size = norm(&turned).max(&scale);
            return Ok(turned.into_iter().map(|x| settled(x, &size)).collect());
        }
        moved(&frame, from, to, x, name)
    })
    .map(Some)
}

// ------------------------------------------------------------------ latitudes and radii

#[derive(Clone, Copy, PartialEq)]
enum Latitude {
    Geodetic,
    Geocentric,
    Parametric,
    Authalic,
    Conformal,
    Rectifying,
    Isometric,
}

impl Latitude {
    fn named(e: &Expr, who: &str) -> Result<Latitude, String> {
        Ok(match plain_word(e).as_deref() {
            Some("geodetic") => Latitude::Geodetic,
            Some("geocentric") => Latitude::Geocentric,
            Some("parametric" | "reduced") => Latitude::Parametric,
            Some("authalic") => Latitude::Authalic,
            Some("conformal") => Latitude::Conformal,
            Some("rectifying") => Latitude::Rectifying,
            Some("isometric") => Latitude::Isometric,
            _ => {
                return Err(format!(
                    "{who}: the kinds of latitude are geodetic, geocentric, parametric (reduced), authalic, conformal, rectifying and isometric; not {e}"
                ));
            }
        })
    }

    /// The latitude of this kind for a geodetic one.
    fn of(self, on: &Ellipsoid, lat: &Float) -> Float {
        let (s, c) = sin_cos(lat);
        let c = c.max(&zero());
        match self {
            Latitude::Geodetic => lat.clone(),
            Latitude::Geocentric => atan2(&(s * (int(1) - on.e2())), &c),
            Latitude::Parametric => atan2(&(s * (int(1) - &on.f)), &c),
            Latitude::Authalic => on.authalic(lat),
            Latitude::Conformal => on.conformal(lat),
            Latitude::Rectifying => on.rectifying(lat),
            Latitude::Isometric => on.isometric(lat),
        }
    }

    /// The geodetic latitude of one of this kind.
    fn back(self, on: &Ellipsoid, x: &Float) -> Float {
        let (s, c) = sin_cos(x);
        let c = c.max(&zero());
        match self {
            Latitude::Geodetic => x.clone(),
            Latitude::Geocentric => atan2(&s, &(c * (int(1) - on.e2()))),
            Latitude::Parametric => atan2(&s, &(c * (int(1) - &on.f))),
            Latitude::Authalic => on.authalic_inverse(x),
            Latitude::Conformal => on.conformal_inverse(x),
            Latitude::Rectifying => on.rectifying_inverse(x),
            Latitude::Isometric => on.isometric_inverse(x),
        }
    }
}

/// The unit of the angles that a last word may say.
fn unit_last(a: &[Expr]) -> (&[Expr], bool) {
    match a.last().and_then(plain_word).and_then(|w| unit_word(&w)) {
        Some(radians) => (&a[..a.len() - 1], radians),
        None => (a, false),
    }
}

fn plain_frame(on: Ellipsoid, radians: bool) -> Frame {
    Frame {
        on,
        arcs: false,
        radians,
        rhumb: false,
    }
}

/// An ellipsoid with a flattening alone: its half axis is 1.
fn flattened(e: &Expr, who: &str) -> Result<Ellipsoid, String> {
    let f = one(e, who)?;
    match (0..1).contains(&f) {
        true => Ok(Ellipsoid { a: int(1), f }),
        false => Err(format!("{who}: a flattening is from 0 up to 1, not {e}")),
    }
}

/// The length of a unit in metres, as a decimal number.
fn unit_length(e: &Expr, who: &str) -> Result<Float, String> {
    let exact = plain_word(e)
        .and_then(|w| metres(&w))
        .ok_or_else(|| format!("{who}: {e} is not a unit of length: {}", units_known()))?;
    real(&exact, who)
}

fn mean_word(e: Option<&Expr>, who: &str) -> Result<bool, String> {
    match e.map(plain_word) {
        None => Ok(true),
        Some(Some(w)) if w == "mean" => Ok(true),
        Some(Some(w)) if w == "norm" => Ok(false),
        Some(_) => Err(format!(
            "{who}: the kind of the mean is \"mean\" or \"norm\""
        )),
    }
}

/// The commands of latitudes of other kinds, of ellipsoids and of their radii.
fn shapes(name: &str, a: &[Expr]) -> Result<Option<Expr>, String> {
    let n = a.len();
    Ok(Some(match name {
        "convertlat" => {
            let (a, radians) = unit_last(a);
            if a.len() != 4 {
                return Err(forms(name));
            }
            let frame = plain_frame(Ellipsoid::of(&a[0], name)?, radians);
            let (from, to) = (Latitude::named(&a[2], name)?, Latitude::named(&a[3], name)?);
            each(&[&a[1]], name, 1, &|x| {
                let lat = match from {
                    Latitude::Isometric => from.back(&frame.on, &x[0]),
                    _ => from.back(&frame.on, &frame.latitude(&x[0], name)?),
                };
                Ok(vec![match to {
                    Latitude::Isometric => to.of(&frame.on, &lat),
                    _ => frame.angle_out(to.of(&frame.on, &lat)),
                }])
            })?
        }
        "geocentriclatitude"
        | "parametriclatitude"
        | "geodeticlatitudefromgeocentric"
        | "geodeticlatitudefromparametric" => {
            let (a, radians) = unit_last(a);
            if a.len() != 2 {
                return Err(forms(name));
            }
            let frame = plain_frame(flattened(&a[1], name)?, radians);
            let kind = match name.contains("geocentric") {
                true => Latitude::Geocentric,
                false => Latitude::Parametric,
            };
            let back = name.starts_with("geodeticlatitude");
            each(&[&a[0]], name, 1, &|x| {
                let lat = frame.latitude(&x[0], name)?;
                Ok(vec![frame.angle_out(match back {
                    true => kind.back(&frame.on, &lat),
                    false => kind.of(&frame.on, &lat),
                })])
            })?
        }
        "referencesphere" => {
            let radius = a
                .first()
                .and_then(plain_word)
                .and_then(|w| SPHERES.iter().find(|(n, _)| *n == w))
                .map(|(_, metres)| dec(metres))
                .ok_or_else(|| {
                    format!(
                        "{name}: the bodies that are taken as spheres are {}",
                        sphere_names()
                    )
                })?;
            match a.get(1) {
                Some(e) => shown(&(radius / unit_length(e, name)?)),
                None => shown(&radius),
            }
        }
        "referenceellipsoid" | "wgs84ellipsoid" => {
            let (named, unit) = match name {
                "wgs84ellipsoid" => (None, a.first()),
                _ => (a.first(), a.get(1)),
            };
            let on = match named {
                None if name == "wgs84ellipsoid" => {
                    Ellipsoid::named("wgs84").expect("a known name")
                }
                Some(e) => plain_word(e)
                    .and_then(|w| Ellipsoid::named(&w))
                    .ok_or_else(|| {
                        format!("{name}: the names are {}; not {e}", ellipsoid_names())
                    })?,
                None => return Err(forms(name)),
            };
            let size = match unit {
                Some(e) => unit_length(e, name)?,
                None => int(1),
            };
            Ellipsoid {
                a: on.a / size,
                f: on.f,
            }
            .vector()
        }
        "meridianarc" | "meridianfwd" => {
            let (v, said) = said_of(a, name, takes(2, 2))?;
            let frame = Frame::of(&said, Some("wgs84"));
            each(&v, name, 1, &|x| {
                let lat = frame.latitude(&x[0], name)?;
                Ok(vec![match name {
                    "meridianarc" => settled(
                        frame.on.meridian(&frame.latitude(&x[1], name)?) - frame.on.meridian(&lat),
                        &frame.on.a,
                    ),
                    _ => {
                        let height = frame.on.meridian(&lat) + &x[1];
                        if fl(height.abs_ref())
                            > frame.on.meridian(&half_pi()) + fl(&frame.on.a * dust())
                        {
                            return Err(format!("{name}: this length goes beyond the pole"));
                        }
                        frame.angle_out(frame.on.rectifying_inverse(
                            &(fl(&height / frame.on.meridian(&half_pi())) * half_pi()),
                        ))
                    }
                }])
            })?
        }
        "spheroid" => {
            if n == 0 || n > 2 {
                return Err(forms(name));
            }
            let on = Ellipsoid::of(&a[0], name)?;
            let (b, e2) = (on.b(), on.e2());
            let third = fl(&on.f / (int(2) - &on.f));
            let area = fl(on.authalic_radius().square()) * pi() * 4u32;
            let volume = fl(on.a.square_ref()) * &b * pi() * 4u32 / 3u32;
            let inverse = match on.round() {
                true => Float::with_val(bits(), rug::float::Special::Infinity),
                false => on.f.clone().recip(),
            };
            let facts: Vec<(&str, Float)> = vec![
                ("semimajoraxis", on.a.clone()),
                ("semiminoraxis", b.clone()),
                ("inverseflattening", inverse),
                ("eccentricity", e2.sqrt()),
                ("flattening", on.f.clone()),
                ("thirdflattening", third),
                ("meanradius", (fl(&on.a * 2u32) + &b) / 3u32),
                ("surfacearea", area),
                ("volume", volume),
            ];
            match a.get(1) {
                None => Expr::list(
                    facts
                        .iter()
                        .map(|(k, x)| Expr::list(vec![crate::texts::text(k), shown(x)]))
                        .collect(),
                ),
                Some(e) => {
                    let wanted = plain_word(e).unwrap_or_default();
                    let wanted = match wanted.as_str() {
                        "a" => "semimajoraxis",
                        "b" => "semiminoraxis",
                        "f" => "flattening",
                        "e" | "ecc" => "eccentricity",
                        "n" => "thirdflattening",
                        other => other,
                    }
                    .to_string();
                    match facts.iter().find(|(k, _)| *k == wanted) {
                        Some((_, x)) => shown(x),
                        None => {
                            return Err(format!(
                                "{name}: the facts of an ellipsoid are {}; not {e}",
                                facts.iter().map(|(k, _)| *k).collect::<Vec<_>>().join(", ")
                            ));
                        }
                    }
                }
            }
        }
        "rcurve" => {
            let (a, radians) = unit_last(a);
            let kinds = ["parallel", "meridian", "transverse"];
            let (kind, rest) = match a.first().and_then(plain_word) {
                Some(w) if kinds.contains(&w.as_str()) => (w, &a[1..]),
                _ => ("parallel".to_string(), a),
            };
            if rest.len() != 2 {
                return Err(forms(name));
            }
            let frame = plain_frame(Ellipsoid::of(&rest[0], name)?, radians);
            each(&[&rest[1]], name, 1, &|x| {
                let (s, c) = sin_cos(&frame.latitude(&x[0], name)?);
                Ok(vec![match kind.as_str() {
                    "parallel" => settled(frame.on.normal(&s) * c.max(&zero()), &frame.on.a),
                    "meridian" => frame.on.along(&s),
                    _ => frame.on.normal(&s),
                }])
            })?
        }
        "rsphere" => {
            let kind = a.first().and_then(plain_word).unwrap_or_default();
            if kind == "euler" {
                let (rest, radians) = unit_last(&a[1..]);
                if rest.len() != 5 {
                    return Err(forms(name));
                }
                let frame = plain_frame(Ellipsoid::of(&rest[4], name)?, radians);
                let x = rest[..4]
                    .iter()
                    .map(|e| one(e, name))
                    .collect::<Result<Vec<_>, _>>()?;
                let (lat1, lon1) = (frame.latitude(&x[0], name)?, frame.angle_in(&x[1]));
                let (lat2, lon2) = (frame.latitude(&x[2], name)?, frame.angle_in(&x[3]));
                // the radius of the surface along the course between the places, in the middle
                let course = way(
                    &Ellipsoid::sphere(int(1)),
                    false,
                    &lat1,
                    &lon1,
                    &lat2,
                    &lon2,
                    false,
                )
                .first;
                let s = (fl(&lat1 + &lat2) / 2u32).sin();
                let (m, across) = (frame.on.along(&s), frame.on.normal(&s));
                let (sc, cc) = sin_cos(&course);
                return Ok(Some(shown(
                    &(fl(&m * &across) / (across * fl(cc.square_ref()) + m * fl(sc.square_ref()))),
                )));
            }
            if n < 2 {
                return Err(forms(name));
            }
            let on = Ellipsoid::of(&a[1], name)?;
            let b = on.b();
            shown(&match (kind.as_str(), n) {
                ("biaxial", 2 | 3) => match mean_word(a.get(2), name)? {
                    true => fl(&on.a + &b) / 2u32,
                    false => fl(&on.a * &b).sqrt(),
                },
                ("triaxial", 2 | 3) => match mean_word(a.get(2), name)? {
                    true => (fl(&on.a * 2u32) + &b) / 3u32,
                    false => (fl(on.a.square_ref()) * &b).cbrt(),
                },
                ("eqavol", 2) => (fl(on.a.square_ref()) * &b).cbrt(),
                ("authalic", 2) => on.authalic_radius(),
                ("rectifying", 2) => on.meridian(&half_pi()) / half_pi(),
                ("curve", 2..=5) => {
                    let (rest, radians) = unit_last(&a[2..]);
                    let frame = plain_frame(on.clone(), radians);
                    let lat = match rest.first() {
                        Some(e) => frame.latitude(&one(e, name)?, name)?,
                        None => pi() / 4u32,
                    };
                    let s = lat.sin();
                    let (m, across) = (on.along(&s), on.normal(&s));
                    match mean_word(rest.get(1), name)? {
                        true => (m + across) / 2u32,
                        false => (m * across).sqrt(),
                    }
                }
                _ => return Err(forms(name)),
            })
        }
        "axes2ecc" | "ecc2flat" | "flat2ecc" | "ecc2n" | "n2ecc" | "majaxis" | "minaxis" => {
            // exact: the half axes a and b, the eccentricity e, the flattening f, n = (a - b)/(a + b)
            let pair = |e: &Expr| {
                e.as_list()
                    .filter(|l| l.len() == 2)
                    .map(|l| (l[0].clone(), l[1].clone()))
            };
            let (x, y) = match (n, a.first().and_then(pair)) {
                (1, Some((x, y))) if matches!(name, "axes2ecc" | "majaxis" | "minaxis") => {
                    (x, Some(y))
                }
                (1, Some((_, ecc))) if matches!(name, "ecc2flat" | "ecc2n") => (ecc, None),
                (1, _) if !matches!(name, "axes2ecc" | "majaxis" | "minaxis") => {
                    (a[0].clone(), None)
                }
                (2, _) if matches!(name, "axes2ecc" | "majaxis" | "minaxis") => {
                    (a[0].clone(), Some(a[1].clone()))
                }
                _ => return Err(forms(name)),
            };
            let one_e = Expr::int(1);
            let sq = |e: &Expr| crate::canon::powi(e.clone(), 2);
            let root = |e: Expr| crate::canon::pow(e, Expr::rat(1, 2));
            let sub = crate::canon::sub;
            let add = |p: Expr, q: Expr| crate::canon::add(vec![p, q]);
            let made = match (name, y) {
                ("axes2ecc", Some(b)) => div(root(sub(sq(&x), sq(&b))), x),
                ("majaxis", Some(e)) => div(x, root(sub(one_e, sq(&e)))),
                ("minaxis", Some(e)) => mul(vec![x, root(sub(one_e, sq(&e)))]),
                ("ecc2flat", _) => sub(one_e.clone(), root(sub(one_e, sq(&x)))),
                ("flat2ecc", _) => root(sub(mul(vec![Expr::int(2), x.clone()]), sq(&x))),
                ("ecc2n", _) => {
                    let r = root(sub(one_e.clone(), sq(&x)));
                    div(sub(one_e.clone(), r.clone()), add(one_e, r))
                }
                ("n2ecc", _) => div(mul(vec![Expr::int(2), root(x.clone())]), add(one_e, x)),
                _ => return Err(forms(name)),
            };
            simplify(&made)
        }
        _ => return Ok(None),
    }))
}

// ------------------------------------------------------------------ units

/// A fraction of a table, as it is written.
fn exact(text: &str) -> Expr {
    let mut parts = text.split('/');
    let top = Expr::integer(
        parts
            .next()
            .unwrap_or("1")
            .parse::<rug::Integer>()
            .expect("a number"),
    );
    match parts.next() {
        Some(bottom) => div(
            top,
            Expr::integer(bottom.parse::<rug::Integer>().expect("a number")),
        ),
        None => top,
    }
}

/// The radians of a unit of angles, when the word is one.
fn angle_size(w: &str) -> Option<Expr> {
    unit_word(w).map(|radians| match radians {
        true => Expr::int(1),
        false => div(Expr::pi(), Expr::int(180)),
    })
}

/// The radius of the sphere in metres: the earth, a body by its name, or a number in the
/// unit of the lengths.
fn radius_metres(given: Option<&Expr>, unit: &Expr, who: &str) -> Result<Expr, String> {
    match given {
        None => Ok(exact("6371000")),
        Some(e) => match plain_word(e) {
            Some(w) => SPHERES
                .iter()
                .find(|(n, _)| *n == w)
                .map(|(_, a)| exact(a))
                .ok_or_else(|| {
                    format!(
                        "{who}: the sphere is a radius or one of {}; not {e}",
                        sphere_names()
                    )
                }),
            None => Ok(mul(vec![e.clone(), unit.clone()])),
        },
    }
}

fn times(x: &Expr, factor: &Expr) -> Expr {
    crate::arrays::map(x, &|v| simplify(&mul(vec![v.clone(), factor.clone()])))
}

/// A unit of a command: the metres of a length or the radians of an angle.
enum Size {
    Length(Expr),
    Angle(Expr),
}

fn size_of(e: &Expr, who: &str) -> Result<Size, String> {
    let w = plain_word(e).ok_or_else(|| format!("{who}: {e} is not a unit"))?;
    match (angle_size(&w), metres(&w)) {
        (Some(k), _) => Ok(Size::Angle(k)),
        (None, Some(m)) => Ok(Size::Length(m)),
        (None, None) => Err(format!(
            "{who}: {e} is not a unit: degrees, radians, {}",
            units_known()
        )),
    }
}

/// A whole fraction as a text with a fixed count of places after the point.
fn fixed(x: &rug::Rational, places: i32) -> String {
    let power = rug::Integer::from(rug::Integer::u_pow_u(10, places.unsigned_abs()));
    let scale = rug::Rational::from(power.clone());
    let moved = match places >= 0 {
        true => rug::Rational::from(x * &scale),
        false => rug::Rational::from(x / &scale),
    };
    let half = rug::Rational::from((1, 2));
    let rounded = rug::Rational::from(moved.abs_ref()) + half;
    let whole = rounded.floor().numer().clone();
    let negative = *x < 0 && whole != 0;
    let digits = match places > 0 {
        true => {
            let text = format!("{whole:0>width$}", width = places as usize + 1);
            let (head, tail) = text.split_at(text.len() - places as usize);
            format!("{head}.{tail}")
        }
        false => (whole * power).to_string(),
    };
    match negative {
        true => format!("-{digits}"),
        false => digits,
    }
}

fn fraction_of(e: &Expr, who: &str) -> Result<rug::Rational, String> {
    match e.as_rat() {
        Some(r) => Ok(r.clone()),
        None => real(e, who)?
            .to_rational()
            .ok_or_else(|| format!("{who}: {e} is not a number")),
    }
}

/// Degrees as whole degrees and minutes (and seconds): the first part that is not 0
/// carries the sign.
fn parted(x: &rug::Rational, seconds: bool) -> Vec<rug::Rational> {
    let size = rug::Rational::from(x.abs_ref());
    let whole = size.clone().floor();
    let minutes = rug::Rational::from(&size - &whole) * 60u32;
    let mut parts = match seconds {
        true => {
            let m = minutes.clone().floor();
            let s = rug::Rational::from(&minutes - &m) * 60u32;
            vec![whole, m, s]
        }
        false => vec![whole, minutes],
    };
    if *x < 0
        && let Some(first) = parts.iter_mut().find(|p| **p != 0)
    {
        *first = rug::Rational::from(-&*first);
    }
    parts
}

fn joined(parts: &[Expr], who: &str) -> Res {
    let values = parts
        .iter()
        .map(|e| fraction_of(e, who))
        .collect::<Result<Vec<_>, _>>()?;
    let first = values.iter().position(|v| *v != 0);
    if let Some(first) = first
        && values[first + 1..].iter().any(|v| *v < 0)
    {
        return Err(format!(
            "{who}: only the first part that is not 0 may be negative"
        ));
    }
    if values[1..]
        .iter()
        .any(|v| rug::Rational::from(v.abs_ref()) >= 60)
    {
        return Err(format!("{who}: minutes and seconds are less than 60"));
    }
    let negative = first.is_some_and(|k| values[k] < 0);
    let mut total = rug::Rational::new();
    let mut scale = rug::Rational::from(1);
    for v in &values {
        total += rug::Rational::from(v.abs_ref()) / &scale;
        scale *= 60u32;
    }
    let total = match negative {
        true => -total,
        false => total,
    };
    Ok(match parts.iter().all(|e| e.as_rat().is_some()) {
        true => Expr::rational(total),
        false => shown(&fl(&total)),
    })
}

/// An angle with its sign as a text.
fn angle_text(x: &rug::Rational, sign: &str, unit: &str, places: i32) -> String {
    let (negative, size) = (*x < 0, rug::Rational::from(x.abs_ref()));
    let shown = match unit {
        "radians" => format!("{} R", fixed(&size, -places)),
        "degrees2dm" => {
            let parts = parted(&size, false);
            format!(
                "{}\u{b0} {}\u{2032}",
                parts[0].numer(),
                padded(&parts[1], -places)
            )
        }
        "degrees2dms" => {
            let parts = parted(&size, true);
            format!(
                "{}\u{b0} {:02}\u{2032} {}\u{2033}",
                parts[0].numer(),
                parts[1].numer(),
                padded(&parts[2], -places)
            )
        }
        _ => format!("{}\u{b0}", fixed(&size, -places)),
    };
    match (sign, negative) {
        ("ns", false) => format!("{shown} N"),
        ("ns", true) => format!("{shown} S"),
        ("ew", false) => format!("{shown} E"),
        ("ew", true) => format!("{shown} W"),
        ("pm", false) => format!("+{shown}"),
        (_, true) => format!("-{shown}"),
        _ => shown,
    }
}

/// Minutes or seconds with two digits before the point.
fn padded(x: &rug::Rational, places: i32) -> String {
    let text = fixed(x, places);
    match text.find('.').unwrap_or(text.len()) < 2 {
        true => format!("0{text}"),
        false => text,
    }
}

/// The degrees of a text like 12°30'15''S, 12-30-15 S, 12d30m15s, 123000S or -12.5.
fn angle_read(text: &str, who: &str) -> Result<rug::Rational, String> {
    let wrong = || {
        format!(
            "{who}: \"{text}\" is not an angle: degrees, minutes and seconds like 12\u{b0}30'15''S, 12-30-15S, 12d30m15s, 123015S (the last four digits are minutes and seconds), or -12.5"
        )
    };
    let trimmed = text.trim();
    let (body, mut negative) = match trimmed.chars().last().map(|c| c.to_ascii_uppercase()) {
        Some('N' | 'E') => (&trimmed[..trimmed.len() - 1], false),
        Some('S' | 'W') => (&trimmed[..trimmed.len() - 1], true),
        _ => (trimmed, false),
    };
    let body = body.trim();
    let body = match body.strip_prefix('-') {
        Some(rest) => {
            negative = !negative;
            rest
        }
        None => body.strip_prefix('+').unwrap_or(body),
    };
    let mut parts = Vec::new();
    let mut number = String::new();
    for c in body.chars() {
        match c {
            '0'..='9' | '.' => number.push(c),
            '\u{b0}' | '\'' | '"' | '-' | ':' | ' ' | 'd' | 'm' | 's' | 'D' | 'M' | 'S'
            | '\u{2032}' | '\u{2033}' => {
                if !number.is_empty() {
                    parts.push(std::mem::take(&mut number));
                }
            }
            _ => return Err(wrong()),
        }
    }
    if !number.is_empty() {
        parts.push(number);
    }
    if parts.is_empty() || parts.len() > 3 {
        return Err(wrong());
    }
    // digits alone before a letter: the last four are minutes and seconds
    let lettered = body.len() < trimmed.len() && !trimmed.starts_with(['-', '+']);
    if let [packed] = parts.as_slice()
        && lettered
        && packed
            .split('.')
            .next()
            .is_some_and(|whole| whole.len() >= 5)
        && body.chars().all(|c| c.is_ascii_digit() || c == '.')
    {
        let whole = packed.split('.').next().unwrap_or("").len();
        let (degrees, rest) = packed.split_at(whole - 4);
        let (minutes, seconds) = rest.split_at(2);
        parts = vec![
            degrees.to_string(),
            minutes.to_string(),
            seconds.to_string(),
        ];
    }
    let mut total = rug::Rational::new();
    let mut scale = rug::Rational::from(1);
    for part in &parts {
        let (whole, fraction) = part.split_once('.').unwrap_or((part, ""));
        if fraction.contains('.') || (whole.is_empty() && fraction.is_empty()) {
            return Err(wrong());
        }
        let digits = format!("{whole}{fraction}");
        let top = digits.parse::<rug::Integer>().map_err(|_| wrong())?;
        let bottom = rug::Integer::from(rug::Integer::u_pow_u(10, fraction.len() as u32));
        total += rug::Rational::from((top, bottom)) / &scale;
        scale *= 60u32;
    }
    Ok(match negative {
        true => -total,
        false => total,
    })
}

/// The letter of the band of latitudes of a UTM zone.
const BANDS: &str = "CDEFGHJKLMNPQRSTUVWX";

/// The number and the letter of the UTM zone of a place in degrees.
fn zone_of(lat: &Float, lon: &Float, who: &str) -> Result<(u32, char), String> {
    let (lat, lon) = (lat.to_f64(), lon.to_f64());
    if !(-80.0..=84.0).contains(&lat) {
        return Err(format!(
            "{who}: the zones of UTM are from 80 degrees south to 84 degrees north"
        ));
    }
    let lon = (lon + 180.0).rem_euclid(360.0) - 180.0;
    let mut zone = ((lon + 180.0) / 6.0).floor() as u32 % 60 + 1;
    if (56.0..64.0).contains(&lat) && (3.0..12.0).contains(&lon) {
        zone = 32;
    }
    if lat >= 72.0 {
        zone = match lon {
            x if (0.0..9.0).contains(&x) => 31,
            x if (9.0..21.0).contains(&x) => 33,
            x if (21.0..33.0).contains(&x) => 35,
            x if (33.0..42.0).contains(&x) => 37,
            _ => zone,
        };
    }
    let band = (((lat + 80.0) / 8.0).floor() as usize).min(BANDS.len() - 1);
    Ok((zone, BANDS.as_bytes()[band] as char))
}

/// The number and the letter of a zone that is written like 31U (a number alone: north).
fn zone_read(e: &Expr, who: &str) -> Result<(u32, char), String> {
    let wrong = || {
        format!(
            "{who}: a zone of UTM is a number from 1 to 60 and a letter from C to X without I and O, like \"31U\"; not {e}"
        )
    };
    let text = match crate::radix::text_of(e) {
        Some(t) => t.trim().to_ascii_uppercase(),
        None => match e.as_i64() {
            Some(k) => k.to_string(),
            None => return Err(wrong()),
        },
    };
    let digits: String = text.chars().take_while(char::is_ascii_digit).collect();
    let letter = text[digits.len()..].trim();
    let zone = digits
        .parse::<u32>()
        .ok()
        .filter(|z| (1..=60).contains(z))
        .ok_or_else(wrong)?;
    let band = match letter {
        "" => 'N',
        l if l.len() == 1 && BANDS.contains(l) => l.chars().next().expect("a letter"),
        _ => return Err(wrong()),
    };
    if band == 'X' && matches!(zone, 32 | 34 | 36) {
        return Err(format!("{who}: there is no zone {zone}X"));
    }
    Ok((zone, band))
}

/// The limits of a zone in degrees: [[south, north], [west, east]].
fn zone_limits(zone: u32, band: char) -> ([i64; 2], [i64; 2]) {
    let k = BANDS.find(band).unwrap_or(0) as i64;
    let lats = match band {
        'X' => [72, 84],
        _ => [-80 + 8 * k, -72 + 8 * k],
    };
    let west = i64::from(zone) * 6 - 186;
    let lons = match (zone, band) {
        (31, 'V') => [0, 3],
        (32, 'V') => [3, 12],
        (31, 'X') => [0, 9],
        (33, 'X') => [9, 21],
        (35, 'X') => [21, 33],
        (37, 'X') => [33, 42],
        _ => [west, west + 6],
    };
    (lats, lons)
}

/// The commands of units of lengths and angles, and of angles as texts.
fn units(name: &str, a: &[Expr]) -> Result<Option<Expr>, String> {
    let n = a.len();
    Ok(Some(match name {
        "km2nm" | "km2sm" | "nm2km" | "nm2sm" | "sm2km" | "sm2nm" | "deg2km" | "km2deg"
        | "deg2nm" | "nm2deg" | "deg2sm" | "sm2deg" | "rad2km" | "km2rad" | "rad2nm" | "nm2rad"
        | "rad2sm" | "sm2rad" => {
            let (from, to) = name.split_once('2').expect("a name with a 2");
            let factor = match (angle_size(from), angle_size(to)) {
                (None, None) if n == 1 => div(
                    metres(from).expect("a length"),
                    metres(to).expect("a length"),
                ),
                (Some(k), None) if n <= 2 => {
                    let unit = metres(to).expect("a length");
                    div(mul(vec![k, radius_metres(a.get(1), &unit, name)?]), unit)
                }
                (None, Some(k)) if n <= 2 => {
                    let unit = metres(from).expect("a length");
                    div(
                        unit.clone(),
                        mul(vec![k, radius_metres(a.get(1), &unit, name)?]),
                    )
                }
                _ => return Err(forms(name)),
            };
            times(&a[0], &simplify(&factor))
        }
        "unitsratio" => match (size_of(&a[0], name)?, size_of(&a[1], name)?) {
            (Size::Length(to), Size::Length(from)) | (Size::Angle(to), Size::Angle(from)) => {
                simplify(&div(from, to))
            }
            _ => {
                return Err(format!(
                    "{name}: a length and an angle have no ratio: distdim takes the radius of a sphere"
                ));
            }
        },
        "earthradius" => match a.first() {
            None => exact("6371000"),
            Some(e) => match size_of(e, name)? {
                Size::Length(unit) => simplify(&div(exact("6371000"), unit)),
                Size::Angle(_) => return Err(format!("{name}: {e} is not a unit of length")),
            },
        },
        "distdim" => {
            let (from, to) = (size_of(&a[1], name)?, size_of(&a[2], name)?);
            let factor = match (from, to) {
                (Size::Length(from), Size::Length(to)) | (Size::Angle(from), Size::Angle(to)) => {
                    div(from, to)
                }
                (Size::Angle(k), Size::Length(unit)) => {
                    div(mul(vec![k, radius_metres(a.get(3), &unit, name)?]), unit)
                }
                (Size::Length(unit), Size::Angle(k)) => div(
                    unit.clone(),
                    mul(vec![k, radius_metres(a.get(3), &unit, name)?]),
                ),
            };
            times(&a[0], &simplify(&factor))
        }
        "angledim" => match (size_of(&a[1], name)?, size_of(&a[2], name)?) {
            (Size::Angle(from), Size::Angle(to)) => times(&a[0], &simplify(&div(from, to))),
            _ => {
                return Err(format!(
                    "{name}: the units of angles are degrees and radians"
                ));
            }
        },
        "todegrees" | "toradians" | "fromdegrees" | "fromradians" => {
            let Size::Angle(said) = size_of(&a[0], name)? else {
                return Err(format!(
                    "{name}: the units of angles are degrees and radians"
                ));
            };
            let other = match name.ends_with("degrees") {
                true => div(Expr::pi(), Expr::int(180)),
                false => Expr::int(1),
            };
            let factor = match name.starts_with("to") {
                true => div(said, other),
                false => div(other, said),
            };
            times(&a[1], &simplify(&factor))
        }
        "wrapto180" | "wrapto360" | "wraptopi" | "wrapto2pi" | "npi2pi" | "zero22pi" => {
            let by_pi = name.ends_with("pi") && !matches!(name, "npi2pi" | "zero22pi");
            let whole_turn = matches!(name, "wrapto360" | "wrapto2pi" | "zero22pi");
            // in half turns: from -1 to 1, or from 0 to 2
            let wrap = |h: rug::Rational| -> rug::Rational {
                let full = |h: &rug::Rational| -> rug::Rational {
                    let two = rug::Rational::from(2);
                    let turns = rug::Rational::from(h / &two).floor();
                    let y: rug::Rational = h - turns * &two;
                    match y == 0 && *h > 0 {
                        true => two,
                        false => y,
                    }
                };
                match whole_turn {
                    true => full(&h),
                    false if rug::Rational::from(h.abs_ref()) <= 1 => h,
                    false => full(&(h + 1u32)) - 1u32,
                }
            };
            crate::arrays::map_result(&a[0], &|x| {
                let halves = match by_pi {
                    true => simplify(&div(x.clone(), Expr::pi())),
                    false => simplify(&div(x.clone(), Expr::int(180))),
                };
                Ok(match halves.as_rat() {
                    Some(h) => {
                        let y = Expr::rational(wrap(h.clone()));
                        simplify(&match by_pi {
                            true => mul(vec![y, Expr::pi()]),
                            false => mul(vec![y, Expr::int(180)]),
                        })
                    }
                    None => {
                        // whole turns are taken off the number as it is written
                        let value = real(x, name)?;
                        let half = match by_pi {
                            true => pi(),
                            false => int(180),
                        };
                        let h = fl(&value / &half);
                        let turns = match whole_turn {
                            true => fl(&h / 2u32).floor(),
                            false if fl(h.abs_ref()) <= 1 => zero(),
                            false => (fl(&h + 1u32) / 2u32).floor(),
                        };
                        let turns = turns
                            .to_integer()
                            .ok_or_else(|| format!("{name}: {x} is not a number"))?;
                        let full = match by_pi {
                            true => mul(vec![Expr::int(2), Expr::pi()]),
                            false => Expr::int(360),
                        };
                        simplify(&crate::canon::sub(
                            x.clone(),
                            mul(vec![Expr::integer(turns), full]),
                        ))
                    }
                })
            })?
        }
        "roundn" => {
            let power = match a.get(1) {
                Some(e) => whole(e, "the power of ten", -1000, 1000)? as i32,
                None => -2,
            };
            let step = {
                let ten = rug::Rational::from(rug::Integer::from(rug::Integer::u_pow_u(
                    10,
                    power.unsigned_abs(),
                )));
                match power >= 0 {
                    true => ten,
                    false => ten.recip(),
                }
            };
            crate::arrays::map_result(&a[0], &|x| {
                let exact = x.as_rat().is_some();
                let value = fraction_of(x, name)?;
                let parts = rug::Rational::from(&value / &step);
                let half = rug::Rational::from((1, 2));
                let rounded = match parts < 0 {
                    true => -(rug::Rational::from(parts.abs_ref()) + &half).floor(),
                    false => (parts + &half).floor(),
                };
                let made = rounded * &step;
                Ok(match exact {
                    true => Expr::rational(made),
                    false => shown(&fl(&made)),
                })
            })?
        }
        "dms2degrees" | "dm2degrees" => {
            let count = match name {
                "dms2degrees" => 3,
                _ => 2,
            };
            let row = |e: &Expr| -> Res {
                match e.as_list() {
                    Some(parts)
                        if parts.len() == count && parts.iter().all(|p| p.as_list().is_none()) =>
                    {
                        joined(parts, name)
                    }
                    _ => Err(forms(name)),
                }
            };
            match a[0].as_list() {
                Some(rows) if rows.iter().all(|r| r.as_list().is_some()) => {
                    Expr::list(rows.iter().map(row).collect::<Result<_, _>>()?)
                }
                _ => row(&a[0])?,
            }
        }
        "degrees2dms" | "degrees2dm" => {
            let seconds = name == "degrees2dms";
            let row = |e: &Expr| -> Res {
                let exact = e.as_rat().is_some();
                let parts = parted(&fraction_of(e, name)?, seconds);
                Ok(Expr::list(
                    parts
                        .into_iter()
                        .map(|p| match exact {
                            true => Expr::rational(p),
                            false => shown(&fl(&p)),
                        })
                        .collect(),
                ))
            };
            match a[0].as_list() {
                Some(items) => Expr::list(items.iter().map(row).collect::<Result<_, _>>()?),
                None => row(&a[0])?,
            }
        }
        "angl2str" => {
            let word_at = |k: usize, usual: &str| -> String {
                a.get(k)
                    .and_then(plain_word)
                    .unwrap_or_else(|| usual.to_string())
            };
            let (sign, unit) = (word_at(1, "none"), word_at(2, "degrees"));
            if !matches!(sign.as_str(), "ns" | "ew" | "pm" | "none")
                || !matches!(
                    unit.as_str(),
                    "degrees" | "degrees2dm" | "degrees2dms" | "radians"
                )
            {
                return Err(forms(name));
            }
            let places = match a.get(3) {
                Some(e) => whole(e, "the power of ten of the last digit", -30, 5)? as i32,
                None => -2,
            };
            crate::arrays::map_result(&a[0], &|x| {
                Ok(crate::texts::text(&angle_text(
                    &fraction_of(x, name)?,
                    &sign,
                    &unit,
                    places,
                )))
            })?
        }
        "str2angle" => crate::arrays::map_result(&a[0], &|x| match crate::radix::text_of(x) {
            Some(text) => Ok(Expr::rational(angle_read(text, name)?)),
            None => Err(forms(name)),
        })?,
        "timezone" => crate::arrays::map_result(&a[0], &|x| {
            let lon = real(x, name)?;
            let lon = match fl(lon.abs_ref()) <= 180 {
                true => lon,
                false => {
                    let moved = fl(&lon + 180u32);
                    fl(&moved - (fl(&moved / 360u32)).floor() * 360u32) - 180u32
                }
            };
            // the hours to add to the time of the zone for that of Greenwich
            let hours = -(fl(&lon / 15u32).round().to_f64() as i64);
            let letter = match hours {
                0 => 'Z',
                -9..=-1 => (b'A' + (-hours - 1) as u8) as char,
                -12..=-10 => (b'K' + (-hours - 10) as u8) as char,
                _ => (b'N' + (hours - 1) as u8) as char,
            };
            Ok(Expr::list(vec![
                Expr::int(hours),
                crate::texts::text(&letter.to_string()),
            ]))
        })?,
        "utmzone" => match n {
            1 => {
                let (zone, band) = zone_read(&a[0], name)?;
                let (lats, lons) = zone_limits(zone, band);
                Expr::list(vec![
                    Expr::list(lats.iter().map(|k| Expr::int(*k)).collect()),
                    Expr::list(lons.iter().map(|k| Expr::int(*k)).collect()),
                ])
            }
            _ => {
                let (zone, band) = zone_of(&one(&a[0], name)?, &one(&a[1], name)?, name)?;
                crate::texts::text(&format!("{zone}{band}"))
            }
        },
        _ => return Ok(None),
    }))
}

// ------------------------------------------------------------------ maps

#[derive(Clone, Copy, PartialEq)]
enum Map {
    Mercator,
    WebMercator,
    Equirectangular,
    Sinusoidal,
    CylindricalEqualArea,
    Mollweide,
    Stereographic,
    Gnomonic,
    Orthographic,
    AzimuthalEquidistant,
    AzimuthalEqualArea,
    TransverseMercator,
    Utm,
    LambertConformal,
    Albers,
    EquidistantConic,
}

const MAPS: &[(&str, Map)] = &[
    ("mercator", Map::Mercator),
    ("webmercator", Map::WebMercator),
    ("pseudomercator", Map::WebMercator),
    ("equirectangular", Map::Equirectangular),
    ("platecarree", Map::Equirectangular),
    ("pcarree", Map::Equirectangular),
    ("eqdcylin", Map::Equirectangular),
    ("sinusoidal", Map::Sinusoidal),
    ("sinusoid", Map::Sinusoidal),
    ("cylindricalequalarea", Map::CylindricalEqualArea),
    ("eqacylin", Map::CylindricalEqualArea),
    ("lambcyln", Map::CylindricalEqualArea),
    ("mollweide", Map::Mollweide),
    ("mollweid", Map::Mollweide),
    ("stereographic", Map::Stereographic),
    ("stereo", Map::Stereographic),
    ("gnomonic", Map::Gnomonic),
    ("orthographic", Map::Orthographic),
    ("ortho", Map::Orthographic),
    ("azimuthalequidistant", Map::AzimuthalEquidistant),
    ("eqdazim", Map::AzimuthalEquidistant),
    ("azimuthalequalarea", Map::AzimuthalEqualArea),
    ("lambertazimuthal", Map::AzimuthalEqualArea),
    ("eqaazim", Map::AzimuthalEqualArea),
    ("transversemercator", Map::TransverseMercator),
    ("tranmerc", Map::TransverseMercator),
    ("utm", Map::Utm),
    ("lambertconformal", Map::LambertConformal),
    ("lambertconformalconic", Map::LambertConformal),
    ("lambert", Map::LambertConformal),
    ("albers", Map::Albers),
    ("albersequalarea", Map::Albers),
    ("eqaconic", Map::Albers),
    ("equidistantconic", Map::EquidistantConic),
    ("eqdconic", Map::EquidistantConic),
];

fn map_names() -> String {
    "mercator, webmercator, equirectangular (pcarree), sinusoidal, cylindricalequalarea (eqacylin), mollweide, \
     stereographic, gnomonic, orthographic, azimuthalequidistant (eqdazim), azimuthalequalarea (eqaazim), \
     transversemercator (tranmerc), utm, lambertconformal (lambert), albers (eqaconic), equidistantconic (eqdconic)"
        .to_string()
}

/// A map: its kind, the body, the place in its middle, its scale, the parallels that keep
/// their length, and what is added to its coordinates.
struct Chart {
    map: Map,
    on: Ellipsoid,
    lat0: Float,
    lon0: Float,
    scale: Float,
    parallels: Vec<Float>,
    east: Float,
    north: Float,
    /// The bits and the coefficients of the series of the transverse map.
    series: RefCell<(u32, Vec<Float>)>,
}

/// The cone of a conic map: its constant, and the distance from its top of a latitude.
struct Cone {
    n: Float,
    c: Float,
}

impl Chart {
    /// The radius of the parallel of a latitude over the half axis.
    fn m(&self, lat: &Float) -> Float {
        let (s, c) = sin_cos(lat);
        c.max(&zero()) / (int(1) - self.on.e2() * fl(s.square_ref())).sqrt()
    }

    fn outside(&self, who: &str) -> String {
        format!("{who}: this place is not on the map")
    }

    fn cone(&self, who: &str) -> Result<Cone, String> {
        let (p1, p2) = (
            &self.parallels[0],
            self.parallels.last().expect("a parallel"),
        );
        let (m1, m2) = (self.m(p1), self.m(p2));
        let one_parallel = fl(p1 - p2).abs() <= dust() * 64u32;
        let on = &self.on;
        let (n, c) = match self.map {
            Map::LambertConformal => {
                let (h1, h2) = (on.isometric(p1), on.isometric(p2));
                let n = match one_parallel {
                    true => p1.clone().sin(),
                    false => fl(&m1 / &m2).ln() / fl(&h2 - &h1),
                };
                let f = fl(&m1 * fl(&n * &h1).exp()) / &n;
                (n, f)
            }
            Map::Albers => {
                let (q1, q2) = (on.q(&p1.clone().sin()), on.q(&p2.clone().sin()));
                let n = match one_parallel {
                    true => p1.clone().sin(),
                    false => (fl(m1.square_ref()) - fl(m2.square_ref())) / fl(&q2 - &q1),
                };
                let c = fl(m1.square_ref()) + fl(&n * &q1);
                (n, c)
            }
            _ => {
                let (d1, d2) = (on.meridian(p1), on.meridian(p2));
                let n = match one_parallel {
                    true => p1.clone().sin(),
                    false => fl(&on.a * fl(&m1 - &m2)) / fl(&d2 - &d1),
                };
                let g = fl(&m1 / &n) + fl(&d1 / &on.a);
                (n, g)
            }
        };
        match n.is_finite() && c.is_finite() && fl(n.abs_ref()) > dust() * 64u32 {
            true => Ok(Cone { n, c }),
            false => Err(format!(
                "{who}: the two parallels of a cone are not the equator alone, not opposite and not at a pole"
            )),
        }
    }

    /// The distance of a latitude from the top of the cone.
    fn cone_radius(&self, cone: &Cone, lat: &Float, who: &str) -> Result<Float, String> {
        let on = &self.on;
        let radius = match self.map {
            Map::LambertConformal => fl(&on.a * &cone.c) * (-fl(&cone.n * on.isometric(lat))).exp(),
            Map::Albers => {
                let inside = fl(&cone.c - fl(&cone.n * on.q(&lat.clone().sin())));
                fl(&on.a * inside.max(&zero()).sqrt()) / &cone.n
            }
            _ => fl(&on.a * &cone.c) - on.meridian(lat),
        };
        match radius.is_finite() {
            true => Ok(radius * &self.scale),
            false => Err(self.outside(who)),
        }
    }

    /// The coefficients of the sines that give the rectifying latitude from the conformal
    /// one, as many as a distance from the middle meridian needs. They fall like the powers
    /// of the third flattening while the sines grow away from the meridian: the small ones
    /// are found with more bits than the session has, so that their share is right.
    fn sines(&self, eta: &Float, who: &str) -> Result<Vec<Float>, String> {
        let third = fl(&self.on.f / (int(2) - &self.on.f));
        if third.is_zero() {
            return Ok(Vec::new());
        }
        let far = fl(eta.abs_ref()).to_f64();
        let room = -third.ln().to_f64() - 2.0 * far;
        if room < 1.0 {
            return Err(format!(
                "{who}: this place is too far from the middle meridian of the map"
            ));
        }
        let count = (f64::from(bits()) * std::f64::consts::LN_2 / room).ceil() as usize + 2;
        let wanted = (f64::from(bits()) * (1.0 + 2.0 * far / room)).ceil() as u32 + 64;
        let known = self.series.borrow().0 >= wanted && self.series.borrow().1.len() >= count;
        if !known {
            // in steps, so that the places of one command share the work
            let wanted = wanted.div_ceil(256) * 256;
            let count = count.div_ceil(16) * 16;
            let before = numeric::digits();
            numeric::set_digits(numeric::digits_for(wanted));
            let size = count + 2;
            let samples: Vec<Float> = (1..size)
                .map(|k| {
                    let chi = half_pi() * k as u32 / size as u32;
                    self.on.rectifying(&self.on.conformal_inverse(&chi)) - chi
                })
                .collect();
            let made: Vec<Float> = (1..=count)
                .map(|j| {
                    let mut sum = zero();
                    for (k, d) in samples.iter().enumerate() {
                        sum += fl(d * (pi() * (j * (k + 1)) as u32 / size as u32).sin());
                    }
                    sum * 2u32 / size as u32
                })
                .collect();
            numeric::set_digits(before);
            *self.series.borrow_mut() = (wanted, made);
        }
        Ok(self.series.borrow().1[..count].to_vec())
    }

    /// The radius of the sphere on which the meridian has its length.
    fn rectifying_radius(&self) -> Float {
        self.on.meridian(&half_pi()) / half_pi()
    }

    fn forward(&self, lat: &Float, lon: &Float, who: &str) -> Result<(Float, Float), String> {
        let on = &self.on;
        let a = &on.a;
        let dl = wrapped(fl(lon - &self.lon0));
        let (sl, cl) = sin_cos(&dl);
        let (x, y) = match self.map {
            Map::Mercator | Map::WebMercator => {
                if at_pole(lat) {
                    return Err(format!("{who}: the poles are not on the map of Mercator"));
                }
                let k = match self.map {
                    Map::Mercator => fl(a * &self.scale) * self.m(&self.parallels[0]),
                    _ => fl(a * &self.scale),
                };
                let height = match self.map {
                    Map::Mercator => on.isometric(lat) - on.isometric(&self.lat0),
                    _ => lat.clone().tan().asinh() - self.lat0.clone().tan().asinh(),
                };
                (fl(&k * &dl), k * height)
            }
            Map::Equirectangular => {
                let k = fl(a * &self.scale) * self.m(&self.parallels[0]);
                (
                    k * &dl,
                    (on.meridian(lat) - on.meridian(&self.lat0)) * &self.scale,
                )
            }
            Map::Sinusoidal => {
                let (s, c) = sin_cos(lat);
                let across = on.normal(&s) * c.max(&zero());
                (
                    across * &dl * &self.scale,
                    (on.meridian(lat) - on.meridian(&self.lat0)) * &self.scale,
                )
            }
            Map::CylindricalEqualArea => {
                let k = self.m(&self.parallels[0]);
                let rise = on.q(&lat.clone().sin()) - on.q(&self.lat0.clone().sin());
                (
                    fl(a * &self.scale) * &k * &dl,
                    fl(a * &self.scale) * rise / (k * 2u32),
                )
            }
            Map::Mollweide => {
                let radius = on.authalic_radius() * &self.scale;
                let target = pi() * on.authalic(lat).sin();
                let mut t = on.authalic(lat);
                if !at_pole(lat) {
                    for _ in 0..2000 {
                        let double = fl(&t * 2u32);
                        let slope = (int(1) + double.clone().cos()) * 2u32;
                        if slope.is_zero() {
                            break;
                        }
                        let step = (fl(&double + double.clone().sin()) - &target) / slope;
                        t -= &step;
                        if quiet(&step, &int(1)) {
                            break;
                        }
                    }
                }
                let (st, ct) = sin_cos(&t);
                let root = int(2).sqrt();
                (
                    fl(&radius * &root) * 2u32 / pi() * &dl * ct.max(&zero()),
                    radius * root * st,
                )
            }
            Map::Stereographic => {
                let k = fl(a * &self.scale) * 2u32;
                if at_pole(&self.lat0) {
                    let e = on.ecc();
                    let north = self.lat0 > 0;
                    let height = match north {
                        true => on.isometric(lat),
                        false => -on.isometric(lat),
                    };
                    let shape = (fl(int(1) + &e).pow(fl(int(1) + &e))
                        * fl(int(1) - &e).pow(fl(int(1) - &e)))
                    .sqrt();
                    let rho = k * (-height).exp() / shape;
                    if !rho.is_finite() {
                        return Err(self.outside(who));
                    }
                    match north {
                        true => (fl(&rho * &sl), -rho * cl),
                        false => (fl(&rho * &sl), rho * cl),
                    }
                } else {
                    let (s, c) = sin_cos(&on.conformal(lat));
                    let (s1, c1) = sin_cos(&on.conformal(&self.lat0));
                    let below = fl(&c1 * (int(1) + fl(&s1 * &s) + fl(&c1 * &c) * &cl));
                    if below <= dust() * 64u32 {
                        return Err(self.outside(who));
                    }
                    let size = k * self.m(&self.lat0) / below;
                    (
                        fl(&size * &c) * &sl,
                        size * (fl(&c1 * &s) - fl(&s1 * &c) * cl),
                    )
                }
            }
            Map::Gnomonic | Map::Orthographic => {
                let (s, c) = sin_cos(lat);
                let c = c.max(&zero());
                let (s1, c1) = sin_cos(&self.lat0);
                let c1 = c1.max(&zero());
                let near = fl(&s1 * &s) + fl(&c1 * &c) * &cl;
                let size = match self.map {
                    Map::Gnomonic if near <= dust() * 64u32 => return Err(self.outside(who)),
                    Map::Gnomonic => near.recip(),
                    _ if near < -dust() * 64u32 => return Err(self.outside(who)),
                    _ => int(1),
                } * a
                    * &self.scale;
                (
                    fl(&size * &c) * &sl,
                    size * (fl(&c1 * &s) - fl(&s1 * &c) * cl),
                )
            }
            Map::AzimuthalEqualArea => {
                let (q, whole) = (on.q(&lat.clone().sin()), on.q(&int(1)));
                if at_pole(&self.lat0) {
                    let north = self.lat0 > 0;
                    let rho = match north {
                        true => fl(&whole - &q),
                        false => fl(&whole + &q),
                    }
                    .max(&zero())
                    .sqrt()
                        * a
                        * &self.scale;
                    match north {
                        true => (fl(&rho * &sl), -rho * cl),
                        false => (fl(&rho * &sl), rho * cl),
                    }
                } else {
                    let (s, c) = sin_cos(&on.authalic(lat));
                    let c = c.max(&zero());
                    let (s1, c1) = sin_cos(&on.authalic(&self.lat0));
                    let radius = fl(a * fl(&whole / 2u32).sqrt());
                    let d = fl(a * self.m(&self.lat0)) / fl(&radius * &c1);
                    let near = int(1) + fl(&s1 * &s) + fl(&c1 * &c) * &cl;
                    if near <= dust() * 64u32 {
                        return Err(self.outside(who));
                    }
                    let size = radius * (int(2) / near).sqrt() * &self.scale;
                    (
                        fl(&size * &d) * &c * &sl,
                        size / d * (fl(&c1 * &s) - fl(&s1 * &c) * cl),
                    )
                }
            }
            Map::AzimuthalEquidistant => {
                let found = way(on, false, &self.lat0, &self.lon0, lat, lon, false);
                let (s, c) = sin_cos(&found.first);
                (
                    fl(&found.length * &s) * &self.scale,
                    found.length * c * &self.scale,
                )
            }
            Map::TransverseMercator | Map::Utm => {
                let (s, c) = sin_cos(&on.conformal(lat));
                let c = c.max(&zero());
                let across = fl(&c * &sl);
                if fl(across.abs_ref()) >= int(1) - dust() {
                    return Err(format!(
                        "{who}: this place is too far from the middle meridian of the map"
                    ));
                }
                let (xi1, eta1) = (atan2(&s, &fl(&c * &cl)), across.atanh());
                let (mut xi, mut eta) = (xi1.clone(), eta1.clone());
                for (j, alpha) in self.sines(&eta1, who)?.iter().enumerate() {
                    let (sj, cj) = sin_cos(&fl(&xi1 * (2 * (j as u32 + 1))));
                    let (shj, chj) = fl(&eta1 * (2 * (j as u32 + 1))).sinh_cosh(Float::new(bits()));
                    xi += fl(alpha * &sj) * chj;
                    eta += fl(alpha * &cj) * shj;
                }
                let size = self.rectifying_radius() * &self.scale;
                (fl(&size * &eta), size * (xi - on.rectifying(&self.lat0)))
            }
            Map::LambertConformal | Map::Albers | Map::EquidistantConic => {
                let cone = self.cone(who)?;
                let (rho, rho0) = (
                    self.cone_radius(&cone, lat, who)?,
                    self.cone_radius(&cone, &self.lat0, who)?,
                );
                let (st, ct) = sin_cos(&fl(&cone.n * &dl));
                (fl(&rho * &st), rho0 - rho * ct)
            }
        };
        match x.is_finite() && y.is_finite() {
            true => Ok((x + &self.east, y + &self.north)),
            false => Err(self.outside(who)),
        }
    }

    fn inverse(&self, x: &Float, y: &Float, who: &str) -> Result<(Float, Float), String> {
        let on = &self.on;
        let a = &on.a;
        let (x, y) = (fl(x - &self.east), fl(y - &self.north));
        // the place of a point of a map that is drawn around its middle
        let around = |c: &Float, rho: &Float, middle: &Float| -> (Float, Float) {
            let (sc, cc) = sin_cos(c);
            let (s1, c1) = sin_cos(middle);
            let c1 = c1.max(&zero());
            if rho.is_zero() {
                return (middle.clone(), zero());
            }
            let up = fl(&cc * &s1) + fl(&y * &sc) * &c1 / rho;
            let east = atan2(&fl(&x * &sc), &(fl(rho * &c1) * &cc - fl(&y * &s1) * &sc));
            (up.min(&int(1)).max(&int(-1)).asin(), east)
        };
        let (lat, east) = match self.map {
            Map::Mercator | Map::WebMercator => {
                let k = match self.map {
                    Map::Mercator => fl(a * &self.scale) * self.m(&self.parallels[0]),
                    _ => fl(a * &self.scale),
                };
                let lat = match self.map {
                    Map::Mercator => {
                        on.isometric_inverse(&(fl(&y / &k) + on.isometric(&self.lat0)))
                    }
                    _ => (fl(&y / &k) + self.lat0.clone().tan().asinh())
                        .sinh()
                        .atan(),
                };
                (lat, x / k)
            }
            Map::Equirectangular => {
                let k = fl(a * &self.scale) * self.m(&self.parallels[0]);
                let height = fl(&y / &self.scale) + on.meridian(&self.lat0);
                if fl(height.abs_ref()) > on.meridian(&half_pi()) + fl(a * dust()) {
                    return Err(self.outside(who));
                }
                (
                    on.rectifying_inverse(&(fl(&height / on.meridian(&half_pi())) * half_pi())),
                    x / k,
                )
            }
            Map::Sinusoidal => {
                let height = fl(&y / &self.scale) + on.meridian(&self.lat0);
                if fl(height.abs_ref()) > on.meridian(&half_pi()) + fl(a * dust()) {
                    return Err(self.outside(who));
                }
                let lat =
                    on.rectifying_inverse(&(fl(&height / on.meridian(&half_pi())) * half_pi()));
                let (s, c) = sin_cos(&lat);
                let across = on.normal(&s) * c.max(&zero()) * &self.scale;
                let east = match across <= fl(a * dust()) {
                    true => zero(),
                    false => x / across,
                };
                (lat, east)
            }
            Map::CylindricalEqualArea => {
                let k = self.m(&self.parallels[0]);
                let q = fl(&y * &k) * 2u32 / fl(a * &self.scale) + on.q(&self.lat0.clone().sin());
                if fl(q.abs_ref()) > on.q(&int(1)) + dust() * 64u32 {
                    return Err(self.outside(who));
                }
                (on.q_inverse(&q).asin(), x / (fl(a * &self.scale) * k))
            }
            Map::Mollweide => {
                let radius = on.authalic_radius() * &self.scale;
                let root = int(2).sqrt();
                let st = fl(&y / fl(&radius * &root));
                if fl(st.abs_ref()) > int(1) + dust() * 64u32 {
                    return Err(self.outside(who));
                }
                let t = st.min(&int(1)).max(&int(-1)).asin();
                let double = fl(&t * 2u32);
                let up = fl(&double + double.clone().sin()) / pi();
                let lat = on.authalic_inverse(&up.min(&int(1)).max(&int(-1)).asin());
                let ct = t.cos();
                let east = match ct <= dust() * 64u32 {
                    true => zero(),
                    false => fl(&x * pi()) / (radius * root * 2u32 * ct),
                };
                (lat, east)
            }
            Map::Stereographic => {
                let k = fl(a * &self.scale) * 2u32;
                let rho = hypot(&x, &y);
                if at_pole(&self.lat0) {
                    let e = on.ecc();
                    let north = self.lat0 > 0;
                    let shape = (fl(int(1) + &e).pow(fl(int(1) + &e))
                        * fl(int(1) - &e).pow(fl(int(1) - &e)))
                    .sqrt();
                    let lat = match rho.is_zero() {
                        true => self.lat0.clone(),
                        false => {
                            let height = -(fl(&rho * shape) / k).ln();
                            match north {
                                true => on.isometric_inverse(&height),
                                false => -on.isometric_inverse(&height),
                            }
                        }
                    };
                    let east = match (rho.is_zero(), north) {
                        (true, _) => zero(),
                        (false, true) => atan2(&x, &fl(-&y)),
                        (false, false) => atan2(&x, &y),
                    };
                    (lat, east)
                } else {
                    let middle = on.conformal(&self.lat0);
                    let c1 = middle.clone().cos();
                    let c = atan2(&fl(&rho * &c1), &(k * self.m(&self.lat0))) * 2u32;
                    let (chi, east) = around(&c, &rho, &middle);
                    (on.conformal_inverse(&chi), east)
                }
            }
            Map::Gnomonic | Map::Orthographic => {
                let radius = fl(a * &self.scale);
                let rho = hypot(&x, &y);
                let share = fl(&rho / &radius);
                let c = match self.map {
                    Map::Gnomonic => share.atan(),
                    _ if share > int(1) + dust() * 64u32 => return Err(self.outside(who)),
                    _ => share.min(&int(1)).asin(),
                };
                around(&c, &rho, &self.lat0)
            }
            Map::AzimuthalEqualArea => {
                let whole = on.q(&int(1));
                let (x, y) = (fl(&x / &self.scale), fl(&y / &self.scale));
                if at_pole(&self.lat0) {
                    let north = self.lat0 > 0;
                    let rho = hypot(&x, &y);
                    let q = fl(&whole - fl(&rho / a).square());
                    if q < -fl(&whole + dust() * 64u32) {
                        return Err(self.outside(who));
                    }
                    let q = q.max(&fl(-&whole));
                    match north {
                        true => (
                            on.q_inverse(&q).asin(),
                            match rho.is_zero() {
                                true => zero(),
                                false => atan2(&x, &fl(-&y)),
                            },
                        ),
                        false => (
                            -on.q_inverse(&q).asin(),
                            match rho.is_zero() {
                                true => zero(),
                                false => atan2(&x, &y),
                            },
                        ),
                    }
                } else {
                    let (s1, c1) = sin_cos(&on.authalic(&self.lat0));
                    let radius = fl(a * fl(&whole / 2u32).sqrt());
                    let d = fl(a * self.m(&self.lat0)) / fl(&radius * &c1);
                    let (px, py) = (fl(&x / &d), fl(&y * &d));
                    let rho = hypot(&px, &py);
                    let share = fl(&rho / fl(&radius * 2u32));
                    if share > int(1) + dust() * 64u32 {
                        return Err(self.outside(who));
                    }
                    if rho.is_zero() {
                        (self.lat0.clone(), zero())
                    } else {
                        let (sc, cc) = sin_cos(&(share.min(&int(1)).asin() * 2u32));
                        let q = fl(&whole * (fl(&cc * &s1) + fl(&py * &sc) * &c1 / &rho));
                        let east = atan2(
                            &fl(&px * &sc),
                            &(fl(&rho * &c1) * &cc - fl(&py * &s1) * &sc),
                        );
                        (on.q_inverse(&q.min(&whole).max(&fl(-&whole))).asin(), east)
                    }
                }
            }
            Map::AzimuthalEquidistant => {
                let length = hypot(&x, &y) / &self.scale;
                let course = match length.is_zero() {
                    true => zero(),
                    false => atan2(&x, &y),
                };
                let (lat, lon, _) = onward(on, false, &self.lat0, &self.lon0, &length, &course);
                (lat, lon - &self.lon0)
            }
            Map::TransverseMercator | Map::Utm => {
                let size = self.rectifying_radius() * &self.scale;
                let (xi, eta) = (fl(&y / &size) + on.rectifying(&self.lat0), fl(&x / &size));
                let sines = self.sines(&(fl(eta.abs_ref()) + 0.02f64), who)?;
                let wanted = Complex::with_val(bits(), (&xi, &eta));
                let mut z = wanted.clone();
                for _ in 0..200 {
                    let mut value = z.clone();
                    let mut slope = Complex::with_val(bits(), 1);
                    for (j, alpha) in sines.iter().enumerate() {
                        let twice = 2 * (j as u32 + 1);
                        let (s, c) =
                            Complex::with_val(bits(), &z * twice).sin_cos(Complex::new(bits()));
                        value += s * alpha;
                        slope += c * alpha * twice;
                    }
                    let step = Complex::with_val(bits(), &value - &wanted) / slope;
                    z -= &step;
                    if quiet(&Float::with_val(bits(), step.abs_ref()), &int(1)) {
                        break;
                    }
                }
                let (s, c) = sin_cos(z.real());
                let (sh, _) = z.imag().clone().sinh_cosh(Float::new(bits()));
                let chi = atan2(&s, &hypot(&sh, &c));
                (on.conformal_inverse(&chi), atan2(&sh, &c))
            }
            Map::LambertConformal | Map::Albers | Map::EquidistantConic => {
                let cone = self.cone(who)?;
                let rho0 = self.cone_radius(&cone, &self.lat0, who)?;
                let sign: i32 = match cone.n < 0 {
                    true => -1,
                    false => 1,
                };
                let rise = fl(&rho0 - &y);
                let rho: Float = hypot(&x, &rise) * sign / &self.scale;
                let turn = atan2(&fl(&x * sign), &fl(&rise * sign));
                let lat = match self.map {
                    Map::LambertConformal => match rho.is_zero() {
                        true => half_pi() * sign,
                        false => {
                            on.isometric_inverse(&(-(fl(&rho / fl(a * &cone.c))).ln() / &cone.n))
                        }
                    },
                    Map::Albers => {
                        let q = (fl(&cone.c - fl(fl(&rho * &cone.n) / a).square())) / &cone.n;
                        if fl(q.abs_ref()) > on.q(&int(1)) + dust() * 64u32 {
                            return Err(self.outside(who));
                        }
                        on.q_inverse(&q).asin()
                    }
                    _ => {
                        let height = fl(a * &cone.c) - rho;
                        if fl(height.abs_ref()) > on.meridian(&half_pi()) + fl(a * dust()) {
                            return Err(self.outside(who));
                        }
                        on.rectifying_inverse(&(fl(&height / on.meridian(&half_pi())) * half_pi()))
                    }
                };
                (lat, turn / cone.n)
            }
        };
        let whole_earth = matches!(
            self.map,
            Map::Mercator
                | Map::WebMercator
                | Map::Equirectangular
                | Map::Sinusoidal
                | Map::CylindricalEqualArea
                | Map::Mollweide
        );
        if whole_earth && fl(east.abs_ref()) > pi() + dust() * 64u32 {
            return Err(self.outside(who));
        }
        match lat.is_finite() && east.is_finite() {
            true => Ok((lat, fl(&self.lon0 + east))),
            false => Err(self.outside(who)),
        }
    }
}

/// A map as a command names it, with what follows the two coordinates.
fn chart(
    kind: &Expr,
    rest: &[Expr],
    first: Option<(&Float, &Float)>,
    who: &str,
) -> Result<(Chart, bool), String> {
    let map = plain_word(kind)
        .and_then(|w| MAPS.iter().find(|(n, _)| *n == w).map(|(_, m)| *m))
        .ok_or_else(|| format!("{who}: the maps are {}; not {kind}", map_names()))?;
    let mut radians = false;
    let mut on: Option<Ellipsoid> = None;
    let (mut origin, mut parallels, mut scale, mut zone): (
        Option<&Expr>,
        Option<&Expr>,
        Option<&Expr>,
        Option<&Expr>,
    ) = (None, None, None, None);
    let (mut east, mut north): (Option<&Expr>, Option<&Expr>) = (None, None);
    let mut k = 0;
    while k < rest.len() {
        let w = plain_word(&rest[k]).ok_or_else(|| forms(who))?;
        if let Some(r) = unit_word(&w) {
            radians = r;
            k += 1;
            continue;
        }
        if let Some(named) = Ellipsoid::named(&w) {
            on = Some(named);
            k += 1;
            continue;
        }
        let value = rest
            .get(k + 1)
            .ok_or_else(|| format!("{who}: a value must follow \"{w}\""))?;
        match w.as_str() {
            "origin" => origin = Some(value),
            "parallels" | "parallel" | "mapparallels" => parallels = Some(value),
            "scale" | "scalefactor" => scale = Some(value),
            "zone" => zone = Some(value),
            "ellipsoid" | "geoid" | "spheroid" | "radius" => on = Some(Ellipsoid::of(value, who)?),
            "falseeasting" => east = Some(value),
            "falsenorthing" => north = Some(value),
            _ => {
                return Err(format!(
                    "{who}: {} is not a choice of a map: \"origin\", [lat, lon]; \"parallels\", [lat1, lat2]; \"scale\", k; \"zone\", \"31U\"; \"ellipsoid\", [half axis, eccentricity] or a name; \"falseeasting\", x; \"falsenorthing\", y; \"radians\"",
                    rest[k]
                ));
            }
        }
        k += 2;
    }
    let angle = |x: &Float| match radians {
        true => x.clone(),
        false => fl(x * pi()) / 180u32,
    };
    let usual = match map {
        Map::Utm | Map::WebMercator => Ellipsoid::named("wgs84").expect("a known name"),
        _ => Ellipsoid::sphere(int(1)),
    };
    let on = on.unwrap_or(usual);
    if matches!(map, Map::Gnomonic | Map::Orthographic) && !on.round() {
        return Err(format!(
            "{who}: this map is of a sphere: give a radius, not an ellipsoid"
        ));
    }
    let (mut lat0, mut lon0) = (zero(), zero());
    if let Some(e) = origin {
        match reals(e, who)?.as_slice() {
            [lat, lon, ..] => (lat0, lon0) = (angle(lat), angle(lon)),
            [lon] => lon0 = angle(lon),
            _ => return Err(format!("{who}: the origin of a map is [lat, lon]")),
        }
        if fl(lat0.abs_ref()) > half_pi() + dust() * 64u32 {
            return Err(format!(
                "{who}: the latitude of the origin is from -90 to 90 degrees"
            ));
        }
        lat0 = lat0.min(&half_pi()).max(&-half_pi());
    }
    let parallels = match parallels {
        Some(e) => {
            let given = reals(e, who)?;
            if given.is_empty()
                || given.len() > 2
                || given
                    .iter()
                    .any(|p| fl(angle(p).abs_ref()) >= half_pi() - dust())
            {
                return Err(format!(
                    "{who}: the parallels of a map are one or two latitudes between the poles"
                ));
            }
            given.iter().map(angle).collect()
        }
        None => match map {
            Map::LambertConformal | Map::Albers | Map::EquidistantConic => {
                vec![pi() / 12u32, pi() * 5u32 / 12u32]
            }
            _ => vec![zero()],
        },
    };
    let mut scale = match scale {
        Some(e) => {
            let k = one(e, who)?;
            if k <= 0 {
                return Err(format!("{who}: the scale of a map is more than 0"));
            }
            k
        }
        None => int(1),
    };
    let (mut east, mut north) = (
        match east {
            Some(e) => one(e, who)?,
            None => zero(),
        },
        match north {
            Some(e) => one(e, who)?,
            None => zero(),
        },
    );
    if map == Map::Utm {
        let (number, band) = match (zone, first) {
            (Some(e), _) => zone_read(e, who)?,
            (None, Some((lat, lon))) => {
                let degrees = |x: &Float| match radians {
                    true => fl(x * 180u32) / pi(),
                    false => x.clone(),
                };
                zone_of(&degrees(lat), &degrees(lon), who)?
            }
            (None, None) => return Err(format!("{who}: the zone must be said: \"zone\", \"31U\"")),
        };
        lat0 = zero();
        lon0 = (int(i64::from(number)) * 6u32 - 183u32) * pi() / 180u32;
        if origin.is_none() && scale == 1 {
            scale = dec("0.9996");
        }
        east += 500_000u32;
        if band < 'N' {
            north += 10_000_000u32;
        }
    }
    Ok((
        Chart {
            map,
            on,
            lat0,
            lon0,
            scale,
            parallels,
            east,
            north,
            series: RefCell::new((0, Vec::new())),
        },
        radians,
    ))
}

/// The commands of maps.
fn maps(name: &str, a: &[Expr]) -> Result<Option<Expr>, String> {
    Ok(Some(match name {
        "projfwd" | "projinv" | "mfwdtran" | "minvtran" => {
            if a.len() < 3 {
                return Err(forms(name));
            }
            let forward = matches!(name, "projfwd" | "mfwdtran");
            let first = match forward {
                true => reals(&a[1], name)?
                    .first()
                    .cloned()
                    .zip(reals(&a[2], name)?.first().cloned()),
                false => None,
            };
            let (chart, radians) = chart(
                &a[0],
                &a[3..],
                first.as_ref().map(|(lat, lon)| (lat, lon)),
                name,
            )?;
            let frame = plain_frame(chart.on.clone(), radians);
            each(&[&a[1], &a[2]], name, 2, &|x| match forward {
                true => {
                    let (px, py) = chart.forward(
                        &frame.latitude(&x[0], name)?,
                        &frame.angle_in(&x[1]),
                        name,
                    )?;
                    let size = fl(&chart.on.a * &chart.scale);
                    Ok(vec![settled(px, &size), settled(py, &size)])
                }
                false => {
                    let (lat, lon) = chart.inverse(&x[0], &x[1], name)?;
                    Ok(vec![frame.angle_out(lat), frame.lon_out(lon)])
                }
            })?
        }
        "deg2utm" => {
            if a.len() != 2 {
                return Err(forms(name));
            }
            let (lat, lon) = (one(&a[0], name)?, one(&a[1], name)?);
            let (zone, band) = zone_of(&lat, &lon, name)?;
            let (chart, _) = chart(&crate::texts::text("utm"), &[], Some((&lat, &lon)), name)?;
            let frame = plain_frame(chart.on.clone(), false);
            let (x, y) =
                chart.forward(&frame.latitude(&lat, name)?, &frame.angle_in(&lon), name)?;
            Expr::list(vec![
                shown(&settled(x, &chart.on.a)),
                shown(&settled(y, &chart.on.a)),
                crate::texts::text(&format!("{zone}{band}")),
            ])
        }
        "utm2deg" => {
            if a.len() != 3 {
                return Err(forms(name));
            }
            let (x, y) = (one(&a[0], name)?, one(&a[1], name)?);
            let (chart, _) = chart(
                &crate::texts::text("utm"),
                &[crate::texts::text("zone"), a[2].clone()],
                None,
                name,
            )?;
            let frame = plain_frame(chart.on.clone(), false);
            let (lat, lon) = chart.inverse(&x, &y, name)?;
            list(&[frame.angle_out(lat), frame.lon_out(lon)])
        }
        _ => return Ok(None),
    }))
}

// ------------------------------------------------------------------ the commands

/// How a command is written.
fn forms(name: &str) -> String {
    let after = "[, ellipsoid][, \"radians\"]";
    let form = match name {
        "distance" | "azimuth" => format!("{name}(lat1, lon1, lat2, lon2{after}), {name}(\"rh\", lat1, lon1, lat2, lon2{after}): an ellipsoid is [half axis, eccentricity], the radius of a sphere, or a name like \"wgs84\""),
        "haversine" => "haversine(lat1, lon1, lat2, lon2[, radius][, \"radians\"])".to_string(),
        "reckon" => format!("reckon(lat, lon, length, course{after}), reckon(\"rh\", lat, lon, length, course{after})"),
        "geoddistance" => format!("geoddistance(lat1, lon1, lat2, lon2{after})"),
        "geodreckon" => format!("geodreckon(lat, lon, length, course{after})"),
        "geodarea" | "areaint" => format!("{name}([latitudes], [longitudes]{after}): three places at least"),
        "areaquad" => format!("areaquad(lat1, lon1, lat2, lon2{after})"),
        "departure" => format!("departure(lon1, lon2, lat{after})"),
        "antipode" => "antipode(lat, lon[, \"radians\"])".to_string(),
        "meanm" => format!("meanm([latitudes], [longitudes]{after})"),
        "track2" => format!("track2(lat1, lon1, lat2, lon2{after}[, places]), track2(\"rh\", ...)"),
        "gcwaypts" => "gcwaypts(lat1, lon1, lat2, lon2[, legs])".to_string(),
        "track1" => format!("track1(lat, lon, course[, length or [from, to]]{after}[, places]), track1(\"rh\", lat, lon, course, length, ...)"),
        "scircle1" => format!("scircle1(lat, lon, radius[, [course1, course2]]{after}[, places])"),
        "scircle2" => format!("scircle2(lat, lon, lat of a place on it, lon of that place[, [course1, course2]]{after}[, places])"),
        "legs" => "legs([latitudes], [longitudes][, \"rh\" | \"gc\"]): two places at least".to_string(),
        "gc2sc" => "gc2sc(lat, lon, course[, \"radians\"])".to_string(),
        "gcxgc" | "rhxrh" => format!("{name}(lat1, lon1, course1, lat2, lon2, course2[, \"radians\"])"),
        "scxsc" => "scxsc(lat1, lon1, radius1, lat2, lon2, radius2[, \"radians\"])".to_string(),
        "gcxsc" => "gcxsc(lat, lon, course, lat of the middle, lon of the middle, radius[, \"radians\"])".to_string(),
        "interpm" => "interpm([latitudes], [longitudes], largest step[, \"gc\" | \"rh\"][, \"radians\"])".to_string(),
        "elevation" => format!("elevation(lat1, lon1, height1, lat2, lon2, height2{after})"),
        "ecefoffset" => format!("ecefoffset(lat1, lon1, height1, lat2, lon2, height2{after})"),
        "lla2ecef" => "lla2ecef([lat, lon, height][, ellipsoid]) or rows of them".to_string(),
        "ecef2lla" => "ecef2lla([x, y, z][, ellipsoid]) or rows of them".to_string(),
        "geodetic2ecef" => format!("geodetic2ecef(lat, lon, height{after})"),
        "ecef2geodetic" => format!("ecef2geodetic(x, y, z{after})"),
        "ecef2enuv" | "ecef2nedv" | "enu2ecefv" | "ned2ecefv" => format!("{name}(u, v, w, lat0, lon0[, \"radians\"])"),
        "enu2aer" | "aer2enu" | "ned2aer" | "aer2ned" | "enu2ned" | "ned2enu" => format!("{name}(the three numbers[, \"radians\"])"),
        "convertlat" => "convertlat(ellipsoid, latitude, \"from\", \"to\"[, \"radians\"]): geodetic, geocentric, parametric, authalic, conformal, rectifying, isometric".to_string(),
        "geocentriclatitude" | "parametriclatitude" | "geodeticlatitudefromgeocentric" | "geodeticlatitudefromparametric" => {
            format!("{name}(latitude, flattening[, \"radians\"])")
        }
        "referenceellipsoid" => format!("{name}(\"name\"[, \"unit of length\"]): {}", ellipsoid_names()),
        "referencesphere" => format!("{name}(\"name\"[, \"unit of length\"]): {}", sphere_names()),
        "meridianarc" => format!("meridianarc(lat1, lat2{after})"),
        "meridianfwd" => format!("meridianfwd(lat, length{after})"),
        "wgs84ellipsoid" => "wgs84ellipsoid([\"unit of length\"])".to_string(),
        "spheroid" => "spheroid(ellipsoid[, \"semimajoraxis\" | \"semiminoraxis\" | \"inverseflattening\" | \"eccentricity\" | \"flattening\" | \"thirdflattening\" | \"meanradius\" | \"surfacearea\" | \"volume\"])".to_string(),
        "rcurve" => "rcurve(\"parallel\" | \"meridian\" | \"transverse\", ellipsoid, latitude[, \"radians\"])".to_string(),
        "rsphere" => "rsphere(\"biaxial\" | \"triaxial\", ellipsoid[, \"mean\" | \"norm\"]), rsphere(\"eqavol\" | \"authalic\" | \"rectifying\", ellipsoid), rsphere(\"curve\", ellipsoid[, latitude[, \"mean\" | \"norm\"]]), rsphere(\"euler\", lat1, lon1, lat2, lon2, ellipsoid)".to_string(),
        "axes2ecc" => "axes2ecc(a, b) or axes2ecc([a, b])".to_string(),
        "majaxis" => "majaxis(b, eccentricity)".to_string(),
        "minaxis" => "minaxis(a, eccentricity)".to_string(),
        "ecc2flat" | "ecc2n" => format!("{name}(eccentricity) or {name}([half axis, eccentricity])"),
        "flat2ecc" => "flat2ecc(flattening)".to_string(),
        "n2ecc" => "n2ecc(n)".to_string(),
        "dms2degrees" => "dms2degrees([degrees, minutes, seconds]) or rows of them".to_string(),
        "dm2degrees" => "dm2degrees([degrees, minutes]) or rows of them".to_string(),
        "angl2str" => "angl2str(angle[, \"ns\" | \"ew\" | \"pm\" | \"none\"[, \"degrees\" | \"degrees2dm\" | \"degrees2dms\" | \"radians\"[, power of ten of the last digit]]])".to_string(),
        "str2angle" => "str2angle(\"12\u{b0}30'15''S\")".to_string(),
        "projfwd" | "mfwdtran" => format!("{name}(\"map\", lat, lon[, \"origin\", [lat, lon]][, \"parallels\", [lat1, lat2]][, \"scale\", k][, \"ellipsoid\", e][, \"zone\", \"31U\"][, \"radians\"]): {}", map_names()),
        "projinv" | "minvtran" => format!("{name}(\"map\", x, y[, the choices of projfwd]): {}", map_names()),
        "deg2utm" => "deg2utm(lat, lon)".to_string(),
        "utm2deg" => "utm2deg(x, y, \"zone\")".to_string(),
        _ if name.contains('2') => format!("{name}(the three numbers, lat0, lon0, height0{after}); with ecef and geodetic alone: {name}(the three numbers{after})"),
        _ => format!("{name}: see --help"),
    };
    match form.starts_with(name) {
        true => format!("{name} is written {form}"),
        false => form,
    }
}

/// The commands of the earth.
pub fn call(name: &str, a: &[Expr]) -> Option<Res> {
    if name == "distance" && a.len() == 2 {
        return None;
    }
    for part in [navigation, solid, shapes, units, maps] {
        match part(name, a) {
            Ok(None) => {}
            Ok(Some(value)) => return Some(Ok(value)),
            Err(e) => return Some(Err(e)),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deg(x: f64) -> Float {
        fl(x) * pi() / 180u32
    }

    fn close(a: &Float, b: &Float, digits: u32) -> bool {
        let size = fl(a.abs_ref()).max(&fl(b.abs_ref())).max(&int(1));
        fl(a - b).abs() <= size * Float::with_val(bits(), Float::i_pow_u(10, digits)).recip()
    }

    fn wgs84() -> Ellipsoid {
        Ellipsoid::named("wgs84").expect("a known name")
    }

    fn numbers(e: &Expr) -> Vec<Float> {
        reals(e, "test").expect("numbers")
    }

    fn run(name: &str, a: Vec<Expr>) -> Expr {
        call(name, &a)
            .expect("a command of the earth")
            .expect("an answer")
    }

    fn num(text: &str) -> Expr {
        shown(&dec(text))
    }

    #[test]
    fn a_way_and_back() {
        let flat = Ellipsoid {
            a: int(1000),
            f: dec("0.3"),
        };
        for on in [wgs84(), flat, Ellipsoid::sphere(int(7))] {
            for (lat1, lon1, lat2, lon2) in [
                (10.0, 20.0, -30.0, 140.0),
                (-45.5, 170.0, 60.25, -100.0),
                (0.0, 0.0, 0.5, 179.7),
                (89.0, 0.0, -89.0, 179.0),
            ] {
                let found = Geodesic { on: &on }.inverse(
                    &deg(lat1),
                    &deg(lon1),
                    &deg(lat2),
                    &deg(lon2),
                    false,
                );
                let (lat, east, course) =
                    Geodesic { on: &on }.direct(&deg(lat1), &found.first, &found.length);
                assert!(
                    close(&lat, &deg(lat2), 28),
                    "{lat1} {lon1} {lat2} {lon2}: {lat}"
                );
                assert!(
                    close(&wrapped(east + deg(lon1) - deg(lon2)), &zero(), 28),
                    "{lat1} {lon1} {lat2} {lon2}"
                );
                assert!(
                    close(&wrapped(course - &found.second), &zero(), 26),
                    "{lat1} {lon1} {lat2} {lon2}"
                );
            }
        }
    }

    #[test]
    fn known_ways() {
        // Karney's nearly antipodal places, and a quarter of the meridian
        let on = wgs84();
        let found = Geodesic { on: &on }.inverse(&zero(), &zero(), &deg(0.5), &deg(179.7), false);
        assert!(
            close(&found.length, &dec("19944127.421"), 10),
            "{}",
            found.length
        );
        assert!(close(&(found.first * 180u32 / pi()), &dec("15.556883"), 7));
        assert!(close(
            &on.meridian(&half_pi()),
            &dec("10001965.7293127228"),
            17
        ));
        // along the equator only up to the place where it stops being the shortest way
        let near = Geodesic { on: &on }.inverse(&zero(), &zero(), &zero(), &deg(179.3), false);
        assert!(close(&near.length, &fl(&on.a * deg(179.3)), 28));
        let far = Geodesic { on: &on }.inverse(&zero(), &zero(), &zero(), &deg(179.5), false);
        assert!(far.length < fl(&on.a * deg(179.5)));
        // at a pole the course is counted from the meridian of the longitude given
        let from = Geodesic { on: &on }.inverse(&half_pi(), &zero(), &zero(), &deg(50.0), false);
        assert!(close(&from.first, &deg(130.0), 28));
        assert!(close(&from.length, &on.meridian(&half_pi()), 28));
    }

    #[test]
    fn areas_of_polygons() {
        let on = wgs84();
        let whole = fl(on.authalic_radius().square()) * pi() * 4u32;
        let (lats, lons) = (
            vec![zero(), half_pi(), zero()],
            vec![zero(), zero(), half_pi()],
        );
        let (area, around) = polygon(&on, &lats, &lons);
        assert!(close(&area, &fl(&whole / 8u32), 26), "{area}");
        assert!(close(
            &around,
            &(on.meridian(&half_pi()) * 2u32 + fl(&on.a * half_pi())),
            26
        ));
        // the other way round, and around a pole
        let (back, _) = polygon(
            &on,
            &[zero(), zero(), half_pi()],
            &[zero(), half_pi(), zero()],
        );
        assert!(close(&back, &area, 26));
        let cap = polygon(
            &on,
            &[deg(60.0), deg(60.0), deg(60.0), deg(60.0)],
            &[zero(), half_pi(), pi(), -half_pi()],
        )
        .0;
        assert!(cap > 0 && cap < fl(&whole / 8u32), "{cap}");
        // a small square is nearly flat
        let side = dec("0.000001");
        let small = polygon(
            &Ellipsoid::sphere(int(1)),
            &[zero(), zero(), side.clone(), side.clone()],
            &[zero(), side.clone(), side.clone(), zero()],
        )
        .0;
        assert!(close(&(small / fl(side.square_ref())), &int(1), 10));
    }

    #[test]
    fn the_transverse_map_keeps_angles() {
        let made = Chart {
            map: Map::TransverseMercator,
            on: wgs84(),
            lat0: zero(),
            lon0: zero(),
            scale: int(1),
            parallels: vec![zero()],
            east: zero(),
            north: zero(),
            series: RefCell::new((0, Vec::new())),
        };
        // on the middle meridian the height is the length of the meridian
        let (x, y) = made
            .forward(&deg(45.0), &zero(), "test")
            .expect("a place of the map");
        assert!(x.is_zero() && close(&y, &made.on.meridian(&deg(45.0)), 28));
        // far from it: a step north and a step east are stretched alike and stay square
        for (lat, lon) in [(10.0, 80.0), (55.0, 30.0), (-35.0, -60.0)] {
            let h = dec("0.000000000001");
            let at = |dlat: i32, dlon: i32| {
                made.forward(
                    &(deg(lat) + fl(&h * dlat)),
                    &(deg(lon) + fl(&h * dlon)),
                    "test",
                )
                .expect("a place of the map")
            };
            let (north, south, east, west) = (at(1, 0), at(-1, 0), at(0, 1), at(0, -1));
            let (s, c) = sin_cos(&deg(lat));
            let up = [
                fl(&north.0 - &south.0) / made.on.along(&s),
                fl(&north.1 - &south.1) / made.on.along(&s),
            ];
            let across = [
                fl(&east.0 - &west.0) / (made.on.normal(&s) * &c),
                fl(&east.1 - &west.1) / (made.on.normal(&s) * &c),
            ];
            let (a, b) = (hypot(&up[0], &up[1]), hypot(&across[0], &across[1]));
            assert!(close(&fl(&a / &b), &int(1), 18), "{lat} {lon}: {a} {b}");
            let square = fl(&up[0] * &across[0]) + fl(&up[1] * &across[1]);
            assert!(
                fl(&square / fl(&a * &b)).abs() < dec("0.000000000000000001"),
                "{lat} {lon}"
            );
            let (lat2, lon2) = made
                .inverse(&at(0, 0).0, &at(0, 0).1, "test")
                .expect("a place of the earth");
            assert!(close(&lat2, &deg(lat), 28) && close(&lon2, &deg(lon), 28));
        }
        assert!(made.forward(&zero(), &deg(89.0), "test").is_err());
    }

    #[test]
    fn places_in_space_and_back() {
        let on = wgs84();
        for (lat, lon, h) in [
            (48.8562, 2.3508, 67.4),
            (-27.2, -65.4, -3899047.25),
            (-50.4, 3.58, 2179702.8),
            (89.999, 10.0, 1e7),
            (0.0, 0.0, 0.0),
        ] {
            let p = ecef(&on, &deg(lat), &deg(lon), &fl(h));
            let (lat2, lon2, h2) = geodetic(&on, &p);
            assert!(
                close(&lat2, &deg(lat), 26) && close(&lon2, &deg(lon), 26),
                "{lat} {lon} {h}"
            );
            assert!(
                fl(&h2 - h).abs() < dec("0.00000000000000000001"),
                "{lat} {lon} {h}: {h2}"
            );
        }
        // east, north and up there and back
        let step = [fl(100), fl(-250), fl(30)];
        let turned = enu_from(
            &deg(46.0),
            &deg(7.75),
            &enu_of(&deg(46.0), &deg(7.75), &step),
        );
        assert!((0..3).all(|k| close(&turned[k], &step[k], 28)));
    }

    #[test]
    fn units_are_exact() {
        let one = Expr::int(1);
        assert_eq!(
            run("nm2sm", vec![one.clone()]).to_string(),
            "1822831/1584000"
        );
        assert_eq!(
            run(
                "unitsratio",
                vec![crate::texts::text("cm"), crate::texts::text("in")]
            )
            .to_string(),
            "127/50"
        );
        assert_eq!(run("deg2km", vec![Expr::int(180)]).to_string(), "6371pi");
        assert_eq!(run("wrapto360", vec![Expr::int(720)]).to_string(), "360");
        assert_eq!(run("wrapto180", vec![Expr::int(-181)]).to_string(), "179");
        assert_eq!(
            run(
                "dms2degrees",
                vec![Expr::list(vec![
                    Expr::int(0),
                    Expr::int(-30),
                    Expr::int(36)
                ])]
            )
            .to_string(),
            "-51/100"
        );
        assert_eq!(
            run("degrees2dms", vec![Expr::rat(-51, 100)]).to_string(),
            "[0, -30, 36]"
        );
        assert_eq!(
            run("str2angle", vec![crate::texts::text("123d30m00sW")]).to_string(),
            "-247/2"
        );
        assert_eq!(
            run("utmzone", vec![Expr::int(60), Expr::int(5)]).to_string(),
            "\"32V\""
        );
        assert_eq!(
            angle_text(&rug::Rational::from((-25, 2)), "ew", "degrees2dms", -2),
            "12\u{b0} 30\u{2032} 00.00\u{2033} W"
        );
        assert_eq!(fixed(&rug::Rational::from((-1, 400)), 2), "0.00");
        assert_eq!(fixed(&rug::Rational::from((12345, 10)), -2), "1200");
    }

    #[test]
    fn commands_agree_with_each_other() {
        let (lat1, lon1, lat2, lon2) = (
            num("52.2297"),
            num("21.0122"),
            num("40.7128"),
            num("-74.006"),
        );
        let name = crate::texts::text("wgs84");
        let length = run(
            "distance",
            vec![
                lat1.clone(),
                lon1.clone(),
                lat2.clone(),
                lon2.clone(),
                name.clone(),
            ],
        );
        let course = run(
            "azimuth",
            vec![
                lat1.clone(),
                lon1.clone(),
                lat2.clone(),
                lon2.clone(),
                name.clone(),
            ],
        );
        let there = numbers(&run(
            "reckon",
            vec![lat1.clone(), lon1.clone(), length, course, name.clone()],
        ));
        assert!(close(&there[0], &dec("40.7128"), 26) && close(&there[1], &dec("-74.006"), 26));
        // a line of one course, and the same with lists
        let rh = crate::texts::text("rh");
        let length = run(
            "distance",
            vec![
                rh.clone(),
                lat1.clone(),
                lon1.clone(),
                lat2.clone(),
                lon2.clone(),
            ],
        );
        let course = run(
            "azimuth",
            vec![
                rh.clone(),
                lat1.clone(),
                lon1.clone(),
                lat2.clone(),
                lon2.clone(),
            ],
        );
        let there = numbers(&run(
            "reckon",
            vec![rh, lat1.clone(), lon1.clone(), length, course],
        ));
        assert!(close(&there[0], &dec("40.7128"), 26) && close(&there[1], &dec("-74.006"), 26));
        let both = run(
            "distance",
            vec![
                Expr::list(vec![lat1.clone(), lat2.clone()]),
                lon1.clone(),
                lat2.clone(),
                lon2.clone(),
            ],
        );
        assert_eq!(numbers(&both).len(), 2);
        // the maps there and back
        for map in [
            "mercator",
            "sinusoidal",
            "eqacylin",
            "mollweide",
            "stereographic",
            "eqdazim",
            "eqaazim",
            "tranmerc",
            "lambert",
            "albers",
            "eqdconic",
        ] {
            let kind = crate::texts::text(map);
            let on = [crate::texts::text("ellipsoid"), name.clone()];
            let point = numbers(&run(
                "projfwd",
                vec![
                    kind.clone(),
                    lat2.clone(),
                    num("-9.5"),
                    on[0].clone(),
                    on[1].clone(),
                ],
            ));
            let back = numbers(&run(
                "projinv",
                vec![
                    kind,
                    shown(&point[0]),
                    shown(&point[1]),
                    on[0].clone(),
                    on[1].clone(),
                ],
            ));
            assert!(
                close(&back[0], &dec("40.7128"), 24) && close(&back[1], &dec("-9.5"), 24),
                "{map}: {back:?}"
            );
        }
        assert!(call("distance", &[lat1.clone(), lon1.clone()]).is_none());
        assert!(
            call("distance", &[lat1.clone(), lon1, num("91"), lon2])
                .expect("a command")
                .is_err()
        );
        assert!(call("sin", &[lat1]).is_none());
    }
}
