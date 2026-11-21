//! Functions of doubles that give the same bits on every system. The library of C that a
//! system has rounds `sin`, `exp`, `pow` and their kind in its own way, a last bit apart from
//! one system to another; here the true value is rounded to the nearest double (MPFR), so
//! that what is printed from doubles is the same everywhere.

use rug::Float;
use rug::ops::Pow;

fn of(x: f64) -> Float {
    Float::with_val(53, x)
}

pub(crate) fn sin(x: f64) -> f64 {
    of(x).sin().to_f64()
}

pub(crate) fn cos(x: f64) -> f64 {
    of(x).cos().to_f64()
}

pub(crate) fn tan(x: f64) -> f64 {
    of(x).tan().to_f64()
}

pub(crate) fn sin_cos(x: f64) -> (f64, f64) {
    (sin(x), cos(x))
}

pub(crate) fn exp(x: f64) -> f64 {
    of(x).exp().to_f64()
}

pub(crate) fn ln(x: f64) -> f64 {
    of(x).ln().to_f64()
}

pub(crate) fn log2(x: f64) -> f64 {
    of(x).log2().to_f64()
}

pub(crate) fn log10(x: f64) -> f64 {
    of(x).log10().to_f64()
}

pub(crate) fn tanh(x: f64) -> f64 {
    of(x).tanh().to_f64()
}

pub(crate) fn asin(x: f64) -> f64 {
    of(x).asin().to_f64()
}

pub(crate) fn acos(x: f64) -> f64 {
    of(x).acos().to_f64()
}

pub(crate) fn atan2(y: f64, x: f64) -> f64 {
    of(y).atan2(&of(x)).to_f64()
}

pub(crate) fn cbrt(x: f64) -> f64 {
    of(x).cbrt().to_f64()
}

pub(crate) fn hypot(x: f64, y: f64) -> f64 {
    of(x).hypot(&of(y)).to_f64()
}

pub(crate) fn pow(x: f64, y: f64) -> f64 {
    of(x).pow(of(y)).to_f64()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nearest_doubles_of_the_true_values() {
        // (the bits, so that a system whose own library rounds another way would show)
        assert_eq!(sin(1.0).to_bits(), 0x3FEA_ED54_8F09_0CEE);
        assert_eq!(cos(1.0).to_bits(), 0x3FE1_4A28_0FB5_068C);
        assert_eq!(exp(1.0).to_bits(), 0x4005_BF0A_8B14_5769);
        assert_eq!(ln(2.0).to_bits(), 0x3FE6_2E42_FEFA_39EF);
        assert_eq!(pow(2.0, 0.5).to_bits(), 0x3FF6_A09E_667F_3BCD);
        assert_eq!(atan2(1.0, 1.0).to_bits(), 0x3FE9_21FB_5444_2D18);
        assert_eq!(cbrt(27.0), 3.0);
        assert_eq!(hypot(3.0, 4.0), 5.0);
        assert_eq!(tanh(0.0), 0.0);
        assert!(ln(-1.0).is_nan());
        assert_eq!(exp(f64::NEG_INFINITY), 0.0);
        assert_eq!(sin_cos(0.0), (0.0, 1.0));
        assert_eq!(log2(8.0), 3.0);
        assert_eq!(log10(1000.0), 3.0);
        assert_eq!(tan(0.0), 0.0);
        assert_eq!(asin(1.0).to_bits(), 0x3FF9_21FB_5444_2D18);
        assert_eq!(acos(1.0), 0.0);
    }
}
