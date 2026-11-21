//! The geoid of the Earth Gravitational Model 1996: its height above the ellipsoid of WGS84
//! from the 130 000 coefficients of the model to degree and order 360 and the coefficients
//! of the correction that turns height anomalies into heights of the geoid, summed as the
//! program F477 of the National Geospatial-Intelligence Agency sums them.

use rug::Float;
use rug::ops::Pow;
use std::sync::OnceLock;

/// The coefficients as GeographicLib packs them: `EGM1996A`, the degree and the order, the
/// C and then the S of the potential, and the same again for the correction (centimetres).
static DATA: &[u8] = include_bytes!("../data/egm96.cof");

struct Model {
    degree: usize,
    c: Vec<f64>,
    s: Vec<f64>,
    correction_c: Vec<f64>,
    correction_s: Vec<f64>,
}

fn model() -> &'static Model {
    static MODEL: OnceLock<Model> = OnceLock::new();
    MODEL.get_or_init(|| {
        let mut at = 8;
        let mut whole = || {
            let v = i32::from_le_bytes(DATA[at..at + 4].try_into().expect("four bytes"));
            at += 4;
            v as usize
        };
        let (degree, order) = (whole(), whole());
        assert_eq!(degree, order, "the model is a triangle");
        let c_size = (order + 1) * (2 * degree - order + 2) / 2;
        let s_size = c_size - (degree + 1);
        let numbers = |count: usize, from: usize| -> Vec<f64> {
            (0..count)
                .map(|k| {
                    let start = from + 8 * k;
                    f64::from_le_bytes(DATA[start..start + 8].try_into().expect("eight bytes"))
                })
                .collect()
        };
        let c = numbers(c_size, at);
        let s = numbers(s_size, at + 8 * c_size);
        let again = at + 8 * (c_size + s_size) + 8;
        let correction_c = numbers(c_size, again);
        let correction_s = numbers(s_size, again + 8 * c_size);
        Model {
            degree,
            c,
            s,
            correction_c,
            correction_s,
        }
    })
}

/// The heights of the geoid of EGM96 above the ellipsoid of WGS84 in metres, for geodetic
/// latitudes and longitudes in degrees: of every pair, or (`grid`) of every latitude with
/// every longitude, row by row.
pub(crate) fn heights(lats: &[Float], lons: &[Float], grid: bool, prec: u32) -> Vec<Float> {
    let m = model();
    let top = m.degree;
    let fl = |x: f64| Float::with_val(prec, x);
    let dec = |text: &str| Float::with_val(prec, Float::parse(text).expect("a number"));
    let radius = fl(6378137.0);
    let squared = dec("0.00669437999013");
    let mass = dec("0.3986004418e15");
    let (gravity, somigliana) = (dec("9.7803253359"), dec("0.00193185265246"));
    // the even zonal terms of the ellipsoid, which the model is measured against
    let zonal: [(usize, Float); 5] = [
        (2, dec("0.108262982131e-2") / fl(5.0).sqrt()),
        (4, dec("-0.237091120053e-5") / fl(9.0).sqrt()),
        (6, dec("0.608346498882e-8") / fl(13.0).sqrt()),
        (8, dec("-0.142681087920e-10") / fl(17.0).sqrt()),
        (10, dec("0.121439275882e-13") / fl(21.0).sqrt()),
    ];
    let place = |n: usize, order: usize| order * top - order * (order.saturating_sub(1)) / 2 + n;
    let pi = Float::with_val(prec, rug::float::Constant::Pi);
    let radians = |degrees: &Float| Float::with_val(prec, degrees * &pi) / 180u32;
    // the factors of the recurrence of the functions of Legendre
    let root = |above: usize, below: usize| (fl(above as f64) / fl(below as f64)).sqrt();
    let mut out = Vec::new();
    let mut before: Option<(Float, Vec<[Float; 4]>, Float)> = None;
    let count = if grid { lats.len() } else { lats.len().min(lons.len()) };
    for i in 0..count {
        let lat = &lats[i];
        // the sums over the degrees for every order: of the potential (cosines, sines) and
        // of the correction; the same again for a latitude that repeats
        let fresh = before.as_ref().is_none_or(|(old, _, _)| old != lat);
        if fresh {
            let (sin, cos) = radians(lat).sin_cos(Float::new(prec));
            let curve = Float::with_val(prec, &radius / (fl(1.0) - Float::with_val(prec, &squared * &sin) * &sin).sqrt());
            let (x, z) = (
                Float::with_val(prec, &curve * &cos),
                Float::with_val(prec, &curve * (fl(1.0) - &squared)) * &sin,
            );
            let far = (Float::with_val(prec, &x * &x) + Float::with_val(prec, &z * &z)).sqrt();
            // the latitude as the middle of the earth sees it
            let (t, u) = (Float::with_val(prec, &z / &far), Float::with_val(prec, &x / &far));
            let normal = Float::with_val(prec, &gravity * (fl(1.0) + Float::with_val(prec, &somigliana * &sin) * &sin))
                / (fl(1.0) - Float::with_val(prec, &squared * &sin) * &sin).sqrt();
            let ratio = Float::with_val(prec, &radius / &far);
            let mut sums: Vec<[Float; 4]> = Vec::with_capacity(top + 1);
            let mut diagonal = fl(1.0);
            for order in 0..=top {
                if order == 1 {
                    diagonal = Float::with_val(prec, &u * fl(3.0).sqrt());
                } else if order > 1 {
                    diagonal = Float::with_val(prec, &diagonal * &u) * root(2 * order + 1, 2 * order);
                }
                let mut sum = [Float::new(prec), Float::new(prec), Float::new(prec), Float::new(prec)];
                let (mut two_back, mut one_back) = (Float::new(prec), diagonal.clone());
                let mut power = ratio.clone().pow(order as u32);
                for n in order..=top {
                    let p = match n - order {
                        0 => diagonal.clone(),
                        1 => Float::with_val(prec, &t * &one_back) * fl((2 * order + 3) as f64).sqrt(),
                        _ => {
                            let a = root(4 * n * n - 1, n * n - order * order);
                            let b = root((n - 1) * (n - 1) - order * order, 4 * (n - 1) * (n - 1) - 1);
                            (Float::with_val(prec, &t * &one_back) - Float::with_val(prec, &b * &two_back)) * a
                        }
                    };
                    let at = place(n, order);
                    if n >= 2 {
                        let mut c = fl(m.c[at]);
                        if order == 0
                            && let Some((_, own)) = zonal.iter().find(|(degree, _)| *degree == n)
                        {
                            c += own;
                        }
                        let scaled = Float::with_val(prec, &p * &power);
                        sum[0] += Float::with_val(prec, &scaled * &c);
                        if order > 0 {
                            sum[1] += Float::with_val(prec, &scaled * fl(m.s[at - (top + 1)]));
                        }
                    }
                    sum[2] += Float::with_val(prec, &p * fl(m.correction_c[at]));
                    if order > 0 {
                        sum[3] += Float::with_val(prec, &p * fl(m.correction_s[at - (top + 1)]));
                    }
                    if n > order {
                        two_back = one_back;
                    }
                    one_back = p;
                    power *= &ratio;
                }
                sums.push(sum);
            }
            let scale = Float::with_val(prec, &mass / Float::with_val(prec, &normal * &far));
            before = Some((lat.clone(), sums, scale));
        }
        let (_, sums, scale) = before.as_ref().expect("the sums of this latitude");
        let along: &[Float] = if grid { lons } else { &lons[i..=i] };
        for lon in along {
            let angle = radians(lon);
            let (mut anomaly, mut correction) = (Float::new(prec), Float::new(prec));
            for (order, sum) in sums.iter().enumerate() {
                let (sin, cos) = Float::with_val(prec, &angle * order as u64).sin_cos(Float::new(prec));
                anomaly += Float::with_val(prec, &sum[0] * &cos) + Float::with_val(prec, &sum[1] * &sin);
                correction += Float::with_val(prec, &sum[2] * &cos) + Float::with_val(prec, &sum[3] * &sin);
            }
            out.push(Float::with_val(prec, &anomaly * scale) + correction / 100u32 - dec("0.53"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_model_is_read_and_summed() {
        let m = model();
        assert_eq!(m.degree, 360);
        assert_eq!(m.c[2], -0.000484165371736);
        // the grid of the agency at its corners: 13.606 m at the north pole, -29.534 m at the south
        // pole, 17.162 m where the equator meets the meridian of Greenwich
        let at = |lat: f64, lon: f64| -> f64 {
            heights(&[Float::with_val(120, lat)], &[Float::with_val(120, lon)], false, 120)[0].to_f64()
        };
        assert!((at(90.0, 0.0) - 13.606).abs() < 0.002, "{}", at(90.0, 0.0));
        assert!((at(-90.0, 0.0) + 29.534).abs() < 0.002, "{}", at(-90.0, 0.0));
        assert!((at(0.0, 0.0) - 17.162).abs() < 0.002, "{}", at(0.0, 0.0));
    }
}
