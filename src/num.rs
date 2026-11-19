use rug::ops::Pow;
use rug::{Complete, Complex, Integer, Rational};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

pub type Int = Integer;
pub type Rat = Rational;

pub fn rat(p: i64, q: i64) -> Rat {
    Rat::from((p, q))
}

/// Exact complex rational number.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Num {
    pub re: Rat,
    pub im: Rat,
}

impl Num {
    pub fn new(re: Rat, im: Rat) -> Self {
        Num { re, im }
    }
    pub fn real(re: Rat) -> Self {
        Num { re, im: Rat::new() }
    }
    pub fn int(n: i64) -> Self {
        Self::real(Rat::from(n))
    }
    pub fn from_int(n: Int) -> Self {
        Self::real(Rat::from(n))
    }
    pub fn frac(p: i64, q: i64) -> Self {
        Self::real(Rat::from((p, q)))
    }
    pub fn zero() -> Self {
        Self::int(0)
    }
    pub fn one() -> Self {
        Self::int(1)
    }
    pub fn i() -> Self {
        Num {
            re: Rat::new(),
            im: Rat::from(1),
        }
    }
    pub fn is_zero(&self) -> bool {
        self.re.is_zero() && self.im.is_zero()
    }
    pub fn is_one(&self) -> bool {
        self.im.is_zero() && self.re == 1
    }
    pub fn is_minus_one(&self) -> bool {
        self.im.is_zero() && self.re == -1
    }
    pub fn is_real(&self) -> bool {
        self.im.is_zero()
    }
    pub fn is_imag(&self) -> bool {
        self.re.is_zero() && !self.im.is_zero()
    }
    pub fn is_int(&self) -> bool {
        self.im.is_zero() && self.re.is_integer()
    }
    pub fn as_int(&self) -> Option<&Int> {
        self.is_int().then(|| self.re.numer())
    }
    pub fn as_i64(&self) -> Option<i64> {
        self.as_int().and_then(|n| n.to_i64())
    }
    pub fn as_rat(&self) -> Option<&Rat> {
        self.is_real().then_some(&self.re)
    }
    pub fn is_positive(&self) -> bool {
        self.im.is_zero() && self.re.is_positive()
    }
    pub fn is_negative(&self) -> bool {
        self.im.is_zero() && self.re.is_negative()
    }
    /// A leading minus sign when printed.
    pub fn is_neg_form(&self) -> bool {
        self.re.is_negative() || (self.re.is_zero() && self.im.is_negative())
    }
    pub fn add(&self, o: &Num) -> Num {
        Num::new((&self.re + &o.re).complete(), (&self.im + &o.im).complete())
    }
    pub fn sub(&self, o: &Num) -> Num {
        Num::new((&self.re - &o.re).complete(), (&self.im - &o.im).complete())
    }
    pub fn neg(&self) -> Num {
        Num::new((-&self.re).complete(), (-&self.im).complete())
    }
    pub fn conj(&self) -> Num {
        Num::new(self.re.clone(), (-&self.im).complete())
    }
    pub fn mul(&self, o: &Num) -> Num {
        if self.im.is_zero() && o.im.is_zero() {
            return Num::real((&self.re * &o.re).complete());
        }
        let re = (&self.re * &o.re).complete() - (&self.im * &o.im).complete();
        let im = (&self.re * &o.im).complete() + (&self.im * &o.re).complete();
        Num::new(re, im)
    }
    pub fn norm(&self) -> Rat {
        (&self.re * &self.re).complete() + (&self.im * &self.im).complete()
    }
    pub fn recip(&self) -> Option<Num> {
        if self.is_zero() {
            return None;
        }
        if self.im.is_zero() {
            return Some(Num::real(self.re.clone().recip()));
        }
        let n = self.norm();
        Some(Num::new(
            (&self.re / &n).complete(),
            (-&self.im).complete() / &n,
        ))
    }
    pub fn div(&self, o: &Num) -> Option<Num> {
        o.recip().map(|r| self.mul(&r))
    }
    pub fn powi(&self, n: i64) -> Option<Num> {
        if n < 0 {
            return self.recip()?.powi(n.checked_neg()?);
        }
        if self.im.is_zero() {
            let e = u32::try_from(n).ok()?;
            return Some(Num::real(self.re.clone().pow(e)));
        }
        let (mut base, mut acc, mut e) = (self.clone(), Num::one(), n);
        while e > 0 {
            if e & 1 == 1 {
                acc = acc.mul(&base);
            }
            e >>= 1;
            if e > 0 {
                base = base.mul(&base);
            }
        }
        Some(acc)
    }
    /// Bits of the largest numerator or denominator.
    pub fn bits(&self) -> u32 {
        [
            self.re.numer(),
            self.re.denom(),
            self.im.numer(),
            self.im.denom(),
        ]
        .iter()
        .map(|x| x.significant_bits())
        .max()
        .unwrap_or(0)
    }
    pub fn to_complex(&self, prec: u32) -> Complex {
        Complex::with_val(prec, (&self.re, &self.im))
    }
}

/// Approximate complex number that remembers how many digits it stands for.
#[derive(Clone, Debug)]
pub struct Flt {
    pub v: Complex,
    pub digits: u32,
}

impl Flt {
    fn key(&self) -> [(Option<(Int, i64)>, bool); 2] {
        [self.v.real(), self.v.imag()].map(|part| {
            let me = part.to_integer_exp().map(|(m, e)| match m.find_one(0) {
                Some(tz) => (m >> tz, i64::from(e) + i64::from(tz)),
                None => (m, 0),
            });
            (me, part.is_sign_negative() && !part.is_zero())
        })
    }
}

impl PartialEq for Flt {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}
impl Eq for Flt {}
impl PartialOrd for Flt {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Flt {
    fn cmp(&self, o: &Self) -> Ordering {
        let part = |a: &rug::Float, b: &rug::Float| {
            if a.is_zero() && b.is_zero() {
                Ordering::Equal
            } else {
                a.total_cmp(b)
            }
        };
        part(self.v.real(), o.v.real()).then_with(|| part(self.v.imag(), o.v.imag()))
    }
}
impl Hash for Flt {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.key().hash(h);
    }
}

/// Decimal literal (`12`, `1.5`, `.5`, `1e-3`) as an exact rational.
pub fn parse_decimal(s: &str) -> Option<Rat> {
    let (mant, exp) = match s.find(['e', 'E']) {
        Some(p) => (&s[..p], s[p + 1..].parse::<i64>().ok()?),
        None => (s, 0),
    };
    let (ip, fp) = mant.split_once('.').unwrap_or((mant, ""));
    if (ip.is_empty() && fp.is_empty()) || !ip.chars().chain(fp.chars()).all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let n = Int::from_str_radix(&format!("{ip}{fp}"), 10).ok()?;
    let e = exp.checked_sub(i64::try_from(fp.len()).ok()?)?;
    if e.unsigned_abs() > 1_000_000 {
        return None;
    }
    let p = Int::from(10).pow(e.unsigned_abs() as u32);
    Some(if e >= 0 {
        Rat::from(n * p)
    } else {
        Rat::from((n, p))
    })
}

/// Significant digits of a decimal literal.
pub fn literal_digits(s: &str) -> u32 {
    let mant = s.split(['e', 'E']).next().unwrap_or(s);
    let digits: String = mant.chars().filter(char::is_ascii_digit).collect();
    digits.trim_start_matches('0').len().max(1) as u32
}

pub fn rat_sqrt(r: &Rat) -> Option<Rat> {
    if r.is_negative() || !r.numer().is_perfect_square() || !r.denom().is_perfect_square() {
        return None;
    }
    Some(Rat::from((
        r.numer().clone().sqrt(),
        r.denom().clone().sqrt(),
    )))
}

/// Principal square root of a complex rational when it is one again.
pub fn gaussian_sqrt(z: &Num) -> Option<Num> {
    if z.im.is_zero() {
        return if z.re.is_negative() {
            rat_sqrt(&(-&z.re).complete()).map(|s| Num::new(Rat::new(), s))
        } else {
            rat_sqrt(&z.re).map(Num::real)
        };
    }
    let m = rat_sqrt(&z.norm())?;
    let u = rat_sqrt(&((&m + &z.re).complete() / 2u32))?;
    let v = rat_sqrt(&((&m - &z.re).complete() / 2u32))?;
    Some(Num::new(u, if z.im.is_negative() { -v } else { v }))
}

/// floor(n^(1/k)) for n >= 0 and whether the root is exact.
pub fn int_root(n: &Int, k: u32) -> Option<(Int, bool)> {
    if n.is_negative() || k == 0 {
        return None;
    }
    let (root, rem) = n.clone().root_rem(Int::new(), k);
    Some((root, rem.is_zero()))
}
