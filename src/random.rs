//! Random numbers from the Mersenne Twister, with the streams of MATLAB's `rand`, `randi`,
//! `randn` and `randperm` for a seed.

use crate::arrays::{LIMIT, whole, word};
use crate::expr::Expr;
use crate::matrix;
use crate::numeric;
use rug::float::Constant;
use rug::{Complex, Float};
use std::cell::RefCell;

type Res = Result<Expr, String>;

struct Twister {
    state: [u32; 624],
    at: usize,
}

impl Twister {
    fn new(seed: u32) -> Twister {
        let mut state = [0u32; 624];
        state[0] = seed;
        for k in 1..624 {
            let prev = state[k - 1];
            state[k] = 1_812_433_253u32
                .wrapping_mul(prev ^ (prev >> 30))
                .wrapping_add(k as u32);
        }
        Twister { state, at: 624 }
    }

    fn next(&mut self) -> u32 {
        if self.at >= 624 {
            for k in 0..624 {
                let y = (self.state[k] & 0x8000_0000) | (self.state[(k + 1) % 624] & 0x7fff_ffff);
                let mut v = self.state[(k + 397) % 624] ^ (y >> 1);
                if y & 1 == 1 {
                    v ^= 0x9908_b0df;
                }
                self.state[k] = v;
            }
            self.at = 0;
        }
        let mut y = self.state[self.at];
        self.at += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }

    /// A whole number below 2^53: the number over 2^53 is uniform from 0 to under 1.
    fn bits53(&mut self) -> u64 {
        let (a, b) = (u64::from(self.next() >> 5), u64::from(self.next() >> 6));
        a * 67_108_864 + b
    }
}

thread_local! {
    static STREAM: RefCell<Option<Twister>> = const { RefCell::new(None) };
}

fn clock_seed() -> u32 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    ((now ^ (now >> 32)) as u32) | 1
}

/// The stream of a seed as MATLAB numbers them: its seed 0 is the twister's 5489.
fn seed(value: u32) {
    let start = if value == 0 { 5489 } else { value };
    STREAM.with(|s| *s.borrow_mut() = Some(Twister::new(start)));
}

fn bits53() -> u64 {
    STREAM.with(|s| {
        s.borrow_mut()
            .get_or_insert_with(|| Twister::new(clock_seed()))
            .bits53()
    })
}

/// Numbers drawn for a search: from the stream of the session once that has begun, else
/// from a stream of their own, which starts the same way every time.
pub(crate) struct Draws {
    own: Option<Twister>,
}

impl Draws {
    pub(crate) fn new() -> Draws {
        let begun = STREAM.with(|s| s.borrow().is_some());
        Draws {
            own: (!begun).then(|| Twister::new(5489)),
        }
    }

    /// Numbers of a stream of their own, as `rng(seed)` begins it.
    pub(crate) fn seeded(seed: u32) -> Draws {
        Draws {
            own: Some(Twister::new(if seed == 0 { 5489 } else { seed })),
        }
    }

    fn bits(&mut self) -> u64 {
        match &mut self.own {
            Some(stream) => stream.bits53(),
            None => bits53(),
        }
    }

    /// A number from 0 to under 1.
    pub(crate) fn unit(&mut self) -> Float {
        Float::with_val(64, self.bits()) >> 53u32
    }

    /// A whole number from 0 to under `span`.
    pub(crate) fn below(&mut self, span: usize) -> usize {
        ((u128::from(self.bits()) * span as u128) >> 53) as usize
    }

    /// A standard normal number.
    pub(crate) fn normal(&mut self) -> Float {
        let first = (Float::with_val(64, self.bits()) + 1u32) >> 53u32;
        let angle = self.unit() * Float::with_val(64, Constant::Pi) * 2u32;
        (first.ln() * -2i32).sqrt() * angle.cos()
    }
}

fn precision() -> u32 {
    numeric::bits_for(numeric::digits()).max(64) + 32
}

fn uniform_float() -> Float {
    Float::with_val(precision(), bits53()) >> 53u32
}

fn as_value(x: Float) -> Expr {
    let prec = x.prec();
    numeric::to_float_expr(Complex::with_val(prec, x), numeric::digits())
}

/// Where the last layer of the ziggurat ends, and the area of every layer.
const ZIGGURAT_END: f64 = 3.654_152_885_361_009;
const ZIGGURAT_AREA: f64 = 4.928_673_233_99e-3;

/// The right ends of the 256 layers of the ziggurat of Marsaglia and Tsang, from the
/// lowest (wider than the curve: its end stands for the tail too) to the top one.
fn layers() -> &'static [f64; 257] {
    static LAYERS: std::sync::OnceLock<[f64; 257]> = std::sync::OnceLock::new();
    LAYERS.get_or_init(|| {
        // (the true values rounded, not those of the system: the same numbers on every system)
        let density = |x: f64| crate::detmath::exp(-0.5 * x * x);
        let mut ends = [0.0; 257];
        ends[0] = ZIGGURAT_AREA / density(ZIGGURAT_END);
        ends[1] = ZIGGURAT_END;
        for k in 2..256 {
            ends[k] = (-2.0
                * crate::detmath::ln(ZIGGURAT_AREA / ends[k - 1] + density(ends[k - 1])))
            .sqrt();
        }
        ends
    })
}

/// A standard normal number as MATLAB's `randn` draws it from the twister: the ziggurat
/// with 256 layers. The layer comes from the top eight bits of the second word of a
/// number of 53 bits, the place in it from the whole number; beside the curve one more
/// number decides, and beyond the last layer two logarithms give the tail.
fn ziggurat(next: &mut dyn FnMut() -> u64) -> f64 {
    const SCALE: f64 = 9_007_199_254_740_992.0;
    let ends = layers();
    let density = |x: f64| crate::detmath::exp(-0.5 * x * x);
    loop {
        let bits = next();
        let layer = 255 - ((bits >> 18) & 0xff) as usize;
        let side = 2.0 * (bits as f64 / SCALE) - 1.0;
        let x = side * ends[layer];
        if x.abs() < ends[layer + 1] {
            return x;
        }
        if layer == 0 {
            loop {
                let a = -crate::detmath::ln((next().max(1)) as f64 / SCALE) / ZIGGURAT_END;
                let b = -crate::detmath::ln((next().max(1)) as f64 / SCALE);
                if b + b > a * a {
                    return if side < 0.0 {
                        -(ZIGGURAT_END + a)
                    } else {
                        ZIGGURAT_END + a
                    };
                }
            }
        }
        let height = next() as f64 / SCALE;
        if density(ends[layer + 1]) + height * (density(ends[layer]) - density(ends[layer + 1]))
            < density(x)
        {
            return x;
        }
    }
}

/// A standard normal number of the stream of the session.
fn normal() -> Float {
    Float::with_val(precision(), ziggurat(&mut bits53))
}

/// A standard normal number, for other modules.
pub fn normal_value() -> Expr {
    as_value(normal())
}

/// A whole number from 0 to under `span`.
fn below(span: u64) -> u64 {
    ((u128::from(bits53()) * u128::from(span)) >> 53) as u64
}

/// The numbers below n in an order of chance, as MATLAB's `randperm` finds it: the order
/// that sorts n numbers of the stream.
fn permutation(n: usize) -> Vec<usize> {
    let keys: Vec<u64> = (0..n).map(|_| bits53()).collect();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|k| keys[*k]);
    order
}

/// The sizes after the first arguments: none is one number, `n` is n by n, `m, n` a matrix;
/// one row is a flat list.
fn filled(sizes: &[Expr], f: &mut dyn FnMut() -> Expr) -> Res {
    let count = |e: &Expr| whole(e, "the size", 0, LIMIT as i64).map(|k| k as usize);
    let (rows, cols) = match sizes {
        [] => return Ok(f()),
        [one] => match one.as_list() {
            Some([r, c]) => (count(r)?, count(c)?),
            Some(_) => return Err("the sizes are [rows, columns]".into()),
            None => (count(one)?, count(one)?),
        },
        [r, c] => (count(r)?, count(c)?),
        _ => return Err("the sizes are rows, columns".into()),
    };
    if rows.saturating_mul(cols) > LIMIT {
        return Err(format!("an array of more than {LIMIT} entries"));
    }
    // (MATLAB fills a matrix down its columns)
    let mut cells = vec![vec![Expr::zero(); cols]; rows];
    for j in 0..cols {
        for row in cells.iter_mut() {
            row[j] = f();
        }
    }
    Ok(match rows {
        0 => Expr::list(Vec::new()),
        1 => Expr::list(cells.remove(0)),
        _ if cols == 0 => Expr::list(Vec::new()),
        _ => matrix::from_rows(cells),
    })
}

/// A random number of the uniform distribution from 0 to under 1, for other modules.
pub fn uniform() -> Expr {
    as_value(uniform_float())
}

fn inner(name: &str, a: &[Expr]) -> Result<Option<Expr>, String> {
    let n = a.len();
    Ok(Some(match (name, n) {
        ("rng", 0) => {
            let s = clock_seed();
            seed(s);
            Expr::int(i64::from(s))
        }
        ("rng", 1) => match (a[0].as_i64(), word(&a[0]).as_deref()) {
            (Some(s), _) if (0..=i64::from(u32::MAX)).contains(&s) => {
                seed(s as u32);
                Expr::int(s)
            }
            (_, Some("default")) => {
                seed(0);
                Expr::zero()
            }
            (_, Some("shuffle")) => {
                let s = clock_seed();
                seed(s);
                Expr::int(i64::from(s))
            }
            _ => {
                return Err(
                    "rng(seed) with a whole number from 0 to 4294967295, rng(\"default\") or rng(\"shuffle\")"
                        .into(),
                );
            }
        },
        ("rand", 0..=2) => filled(a, &mut || as_value(uniform_float()))?,
        ("randn", 0..=2) => filled(a, &mut || as_value(normal()))?,
        ("randi", 1..=3) => {
            let end = |e: &Expr| whole(e, "an end of the range", i64::MIN / 4, i64::MAX / 4);
            let (lo, hi) = match a[0].as_list() {
                Some([lo, hi]) => (end(lo)?, end(hi)?),
                Some(_) => return Err("randi([low, high])".into()),
                None => (1, end(&a[0])?),
            };
            if hi < lo {
                return Err(format!("randi: the range from {lo} to {hi} is empty"));
            }
            let span = (hi - lo) as u64 + 1;
            filled(&a[1..], &mut || Expr::int(lo + below(span) as i64))?
        }
        ("datasample", 2) => {
            let items = a[0]
                .as_list()
                .filter(|l| !l.is_empty())
                .ok_or("datasample(list, k): k entries of a list that is not empty")?;
            let take = whole(&a[1], "the number taken", 0, LIMIT as i64)? as usize;
            Expr::list(
                (0..take)
                    .map(|_| items[below(items.len() as u64) as usize].clone())
                    .collect(),
            )
        }
        ("randperm", 1 | 2) => {
            let size = whole(&a[0], "the number of entries", 0, LIMIT as i64)? as usize;
            let take = if n == 2 {
                whole(&a[1], "the number taken", 0, size as i64)? as usize
            } else {
                size
            };
            Expr::list(
                permutation(size)
                    .into_iter()
                    .take(take)
                    .map(|k| Expr::int(k as i64 + 1))
                    .collect(),
            )
        }
        ("randsample", 2) => {
            let pool: Vec<Expr> = match a[0].as_list() {
                Some(items) => items.to_vec(),
                None => (1..=whole(&a[0], "the number of entries", 0, LIMIT as i64)?)
                    .map(Expr::int)
                    .collect(),
            };
            let take = whole(&a[1], "the number taken", 0, pool.len() as i64)? as usize;
            Expr::list(
                permutation(pool.len())
                    .into_iter()
                    .take(take)
                    .map(|k| pool[k].clone())
                    .collect(),
            )
        }
        ("shuffle", 1) => {
            let items = a[0].as_list().ok_or("shuffle needs a list")?;
            Expr::list(
                permutation(items.len())
                    .into_iter()
                    .map(|k| items[k].clone())
                    .collect(),
            )
        }
        _ => return Ok(None),
    }))
}

pub fn call(name: &str, a: &[Expr]) -> Option<Res> {
    inner(name, a).transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_twister_gives_its_known_numbers() {
        // the first outputs of mt19937ar for the seed 5489
        let mut t = Twister::new(5489);
        assert_eq!(t.next(), 3_499_211_612);
        assert_eq!(t.next(), 581_869_302);
        assert_eq!(t.next(), 3_890_346_734);
        // MATLAB's first rand after rng(0): 0.8147236863931789
        let mut t = Twister::new(5489);
        let first = t.bits53() as f64 / 9_007_199_254_740_992.0;
        assert_eq!(first, 0.814_723_686_393_178_9);
        // and after rng(1): 0.417022004702574
        let mut t = Twister::new(1);
        let first = t.bits53() as f64 / 9_007_199_254_740_992.0;
        assert!((first - 0.417_022_004_702_574).abs() < 1e-15);
    }

    #[test]
    fn randn_and_randperm_follow_the_streams_of_matlab() {
        // the first hundred of MATLAB's randn after rng default (its documentation prints
        // them with four decimals); the 52nd is one that the curve decided
        let known = "0.5377 1.8339 -2.2588 0.8622 0.3188 -1.3077 -0.4336 0.3426 3.5784 2.7694 -1.3499 3.0349 0.7254 -0.0631 0.7147 -0.2050 -0.1241 1.4897 1.4090 1.4172 \
            0.6715 -1.2075 0.7172 1.6302 0.4889 1.0347 0.7269 -0.3034 0.2939 -0.7873 0.8884 -1.1471 -1.0689 -0.8095 -2.9443 1.4384 0.3252 -0.7549 1.3703 -1.7115 \
            -0.1022 -0.2414 0.3192 0.3129 -0.8649 -0.0301 -0.1649 0.6277 1.0933 1.1093 -0.8637 0.0774 -1.2141 -1.1135 -0.0068 1.5326 -0.7697 0.3714 -0.2256 1.1174 \
            -1.0891 0.0326 0.5525 1.1006 1.5442 0.0859 -1.4916 -0.7423 -1.0616 2.3505 -0.6156 0.7481 -0.1924 0.8886 -0.7648 -1.4023 -1.4224 0.4882 -0.1774 -0.1961 \
            1.4193 0.2916 0.1978 1.5877 -0.8045 0.6966 0.8351 -0.2437 0.2157 -1.1658 -1.1480 0.1049 0.7223 2.5855 -0.6669 0.1873 -0.0825 -1.9330 -0.4390 -1.7947";
        let mut t = Twister::new(5489);
        for (k, text) in known.split_whitespace().enumerate() {
            let (drawn, wanted) = (
                ziggurat(&mut || t.bits53()),
                text.parse::<f64>().expect("a number"),
            );
            assert!(
                (drawn - wanted).abs() < 5.1e-5,
                "the {}th: {drawn} against {wanted}",
                k + 1
            );
        }
        // after rng(22)
        let mut t = Twister::new(22);
        for wanted in [-1.1193, -0.0729, -0.2847, 1.5111, -1.5112] {
            let drawn = ziggurat(&mut || t.bits53());
            assert!((drawn - wanted).abs() < 5.1e-5, "{drawn} against {wanted}");
        }
        // many of them: the mean 0, the deviation 1, and the tail is there
        let mut t = Twister::new(7);
        let all: Vec<f64> = (0..400_000).map(|_| ziggurat(&mut || t.bits53())).collect();
        let mean = all.iter().sum::<f64>() / all.len() as f64;
        let spread =
            (all.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / all.len() as f64).sqrt();
        let beyond = all.iter().filter(|x| x.abs() > ZIGGURAT_END).count();
        assert!(
            mean.abs() < 0.006 && (spread - 1.0).abs() < 0.005,
            "{mean} {spread}"
        );
        // (2 (1 - Phi(3.654)) = 2.58e-4 of them: about 103)
        assert!(
            (60..150).contains(&beyond),
            "{beyond} beyond the last layer"
        );
        // randperm(6) and randperm(8, 4) of MATLAB's documentation
        seed(0);
        assert_eq!(
            permutation(6).iter().map(|k| k + 1).collect::<Vec<_>>(),
            [6, 3, 5, 1, 2, 4]
        );
        seed(0);
        assert_eq!(
            permutation(8)
                .iter()
                .take(4)
                .map(|k| k + 1)
                .collect::<Vec<_>>(),
            [6, 3, 7, 8]
        );
    }
}
