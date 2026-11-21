//! Networks of layers that slide windows over pictures (convolutions, pooling), and the
//! training that takes the observations a few at a time: steps down the slope with
//! momentum, of Adam, of RMSProp. A network of this kind is a list of layers that begins
//! with `["input", [height, width, channels]]`; `dlnetwork` gives its windows their first
//! weights, the same on every machine.

use crate::arrays::whole;
use crate::expr::Expr;
use crate::neural::{
    Dense, Shuffle, dense, fl, is_net, net_of, net_shown, real, shown, slope, soft, text,
    unit_name, usual, value, zero,
};
use crate::texts::text as written;
use rug::Float;

type Res = Result<Expr, String>;

/// Values that go through a network: channels of rows of columns, or a plain list.
#[derive(Clone)]
struct Values {
    c: usize,
    h: usize,
    w: usize,
    flat: bool,
    v: Vec<Float>,
}

impl Values {
    fn like(&self, v: Vec<Float>) -> Values {
        Values { v, ..self.clone() }
    }

    fn at(&self, c: usize, i: usize, j: usize) -> &Float {
        &self.v[(c * self.h + i) * self.w + j]
    }
}

#[derive(Clone)]
enum Layer {
    Input {
        c: usize,
        h: usize,
        w: usize,
        flat: bool,
    },
    /// the weights of every window: filters by channels by rows by columns
    Conv {
        w: Vec<Float>,
        b: Vec<Float>,
        filters: usize,
        c: usize,
        kh: usize,
        kw: usize,
        stride: [usize; 2],
        pad: [usize; 2],
    },
    Pool {
        max: bool,
        size: [usize; 2],
        stride: [usize; 2],
    },
    Flatten,
    Full {
        w: Vec<Float>,
        b: Vec<Float>,
        out: usize,
        inp: usize,
    },
    Unit {
        f: &'static str,
        extra: Option<Float>,
    },
}

fn int(k: usize) -> Expr {
    Expr::int(k as i64)
}

fn pair(e: &Expr, what: &str, low: i64) -> Result<[usize; 2], String> {
    let items = match e.as_list() {
        Some(l) => l.to_vec(),
        None => vec![e.clone(), e.clone()],
    };
    match &items[..] {
        [a, b] => Ok([
            whole(a, what, low, 4096)? as usize,
            whole(b, what, low, 4096)? as usize,
        ]),
        _ => Err(format!("{what}: one number or [down, across], not {e}")),
    }
}

fn pair_shown(p: [usize; 2]) -> Expr {
    Expr::list(vec![int(p[0]), int(p[1])])
}

fn numbers(e: &Expr, who: &str) -> Result<Vec<Float>, String> {
    fn walk(e: &Expr, who: &str, out: &mut Vec<Float>) -> Result<(), String> {
        match e.as_list() {
            Some(items) => items.iter().try_for_each(|x| walk(x, who, out)),
            None => {
                out.push(real(e, who)?);
                Ok(())
            }
        }
    }
    let mut out = Vec::new();
    walk(e, who, &mut out)?;
    Ok(out)
}

/// The sizes of nested lists, when they are even.
fn sizes(e: &Expr) -> Vec<usize> {
    let mut out = Vec::new();
    let mut now = e;
    while let Some(items) = now.as_list() {
        out.push(items.len());
        match items.first() {
            Some(first) => now = first,
            None => break,
        }
    }
    out
}

fn nest(v: &[Float], dims: &[usize]) -> Expr {
    match dims {
        [] | [_] => Expr::list(v.iter().map(shown).collect()),
        [first, rest @ ..] => {
            let step = v.len() / first.max(&1);
            Expr::list(
                (0..*first)
                    .map(|k| nest(&v[k * step..(k + 1) * step], rest))
                    .collect(),
            )
        }
    }
}

const USAGE: &str = "a network of layers begins with [\"input\", [height, width, channels]] (or [\"input\", n] for n values); then [\"conv\", weights, biases, stride, padding], [\"maxpool\", size, stride], [\"avgpool\", size, stride], [\"flatten\"], [\"dense\", weights, biases] and the functions of units as [\"relu\"], [\"tanh\"], [\"softmax\"]; dlnetwork([imageinputlayer(...), convolution2dlayer(...), ...]) makes one";

/// Whether a value is a network of this kind (its first layer says what comes in).
pub(crate) fn is_convnet(e: &Expr) -> bool {
    e.as_list()
        .and_then(|l| l.first())
        .and_then(Expr::as_list)
        .is_some_and(|first| first.first().and_then(text).as_deref() == Some("input"))
}

fn read_net(e: &Expr, who: &str) -> Result<Vec<Layer>, String> {
    let wrong = || format!("{who}: {USAGE}");
    let mut out = Vec::new();
    let mut shape: Option<(usize, usize, usize, bool)> = None;
    for item in e.as_list().ok_or_else(wrong)? {
        let parts = item.as_list().ok_or_else(wrong)?;
        let kind = parts.first().and_then(text).ok_or_else(wrong)?;
        let layer = match (kind.as_str(), parts.len()) {
            ("input", 2) => {
                let dims: Vec<Expr> = match parts[1].as_list() {
                    Some(l) => l.to_vec(),
                    None => vec![parts[1].clone()],
                };
                let count = |k: usize| {
                    whole(&dims[k], "a size of what comes in", 1, 1 << 20).map(|v| v as usize)
                };
                match dims.len() {
                    1 => Layer::Input {
                        c: count(0)?,
                        h: 1,
                        w: 1,
                        flat: true,
                    },
                    2 => Layer::Input {
                        c: 1,
                        h: count(0)?,
                        w: count(1)?,
                        flat: false,
                    },
                    3 => Layer::Input {
                        c: count(2)?,
                        h: count(0)?,
                        w: count(1)?,
                        flat: false,
                    },
                    _ => return Err(wrong()),
                }
            }
            ("conv", 5) => {
                let dims = sizes(&parts[1]);
                let [filters, c, kh, kw] = dims[..] else {
                    return Err(format!(
                        "{who}: the weights of a layer of windows are filters of channels of matrices"
                    ));
                };
                let w = numbers(&parts[1], who)?;
                let b = numbers(&parts[2], who)?;
                if w.len() != filters * c * kh * kw || b.len() != filters || w.is_empty() {
                    return Err(format!(
                        "{who}: a layer of windows has a bias for every filter and windows of one size"
                    ));
                }
                Layer::Conv {
                    w,
                    b,
                    filters,
                    c,
                    kh,
                    kw,
                    stride: pair(&parts[3], "the stride", 1)?,
                    pad: pair(&parts[4], "the padding", 0)?,
                }
            }
            ("maxpool" | "avgpool", 3) => Layer::Pool {
                max: kind == "maxpool",
                size: pair(&parts[1], "the size of the window", 1)?,
                stride: pair(&parts[2], "the stride", 1)?,
            },
            ("flatten", 1) => Layer::Flatten,
            ("dense", 3) => {
                let dims = sizes(&parts[1]);
                let [out_count, inp] = dims[..] else {
                    return Err(format!(
                        "{who}: the weights of a dense layer are a matrix, a row for every unit"
                    ));
                };
                let (w, b) = (numbers(&parts[1], who)?, numbers(&parts[2], who)?);
                if w.len() != out_count * inp || b.len() != out_count || w.is_empty() {
                    return Err(format!(
                        "{who}: a dense layer has a row of weights and a bias for every unit"
                    ));
                }
                Layer::Full {
                    w,
                    b,
                    out: out_count,
                    inp,
                }
            }
            (name, 1 | 2) => {
                let f = unit_name(name)
                    .filter(|f| !matches!(*f, "compet" | "rbf"))
                    .ok_or_else(wrong)?;
                let extra = match parts.get(1).cloned().or_else(|| usual(f)) {
                    Some(x) => Some(real(&x, who)?),
                    None => None,
                };
                Layer::Unit { f, extra }
            }
            _ => return Err(wrong()),
        };
        // the sizes must go together
        shape = Some(match (&layer, shape) {
            (Layer::Input { c, h, w, flat }, None) => (*c, *h, *w, *flat),
            (_, None) | (Layer::Input { .. }, Some(_)) => return Err(wrong()),
            (
                Layer::Conv {
                    filters,
                    c,
                    kh,
                    kw,
                    stride,
                    pad,
                    ..
                },
                Some((cc, h, w, false)),
            ) => {
                if *c != cc || h + 2 * pad[0] < *kh || w + 2 * pad[1] < *kw {
                    return Err(format!(
                        "{who}: a layer of windows for {c} channels and windows of {kh} by {kw} gets {cc} channels of {h} by {w}"
                    ));
                }
                (
                    *filters,
                    (h + 2 * pad[0] - kh) / stride[0] + 1,
                    (w + 2 * pad[1] - kw) / stride[1] + 1,
                    false,
                )
            }
            (Layer::Pool { size, stride, .. }, Some((c, h, w, false))) => {
                if h < size[0] || w < size[1] {
                    return Err(format!(
                        "{who}: a window of {} by {} is larger than the {h} by {w} values it lies on",
                        size[0], size[1]
                    ));
                }
                (
                    c,
                    (h - size[0]) / stride[0] + 1,
                    (w - size[1]) / stride[1] + 1,
                    false,
                )
            }
            (Layer::Flatten, Some((c, h, w, _))) => (c * h * w, 1, 1, true),
            (Layer::Full { out: o, inp, .. }, Some((c, 1, 1, true))) => {
                if *inp != c {
                    return Err(format!("{who}: a dense layer for {inp} values gets {c}"));
                }
                (*o, 1, 1, true)
            }
            (Layer::Unit { f, .. }, Some(s)) => {
                if *f == "softmax" && !s.3 {
                    return Err(format!(
                        "{who}: softmax takes a list of values: a [\"flatten\"] layer goes before it"
                    ));
                }
                s
            }
            (Layer::Conv { .. } | Layer::Pool { .. }, Some((_, _, _, true))) => {
                return Err(format!(
                    "{who}: a layer of windows takes channels of matrices, not a list of values"
                ));
            }
            (Layer::Full { .. }, Some(_)) => {
                return Err(format!(
                    "{who}: a dense layer takes a list of values: a [\"flatten\"] layer goes before it"
                ));
            }
        });
        out.push(layer);
    }
    if out.len() < 2 {
        return Err(wrong());
    }
    Ok(out)
}

fn show_net(net: &[Layer]) -> Expr {
    Expr::list(
        net.iter()
            .map(|l| {
                Expr::list(match l {
                    Layer::Input { c, flat: true, .. } => vec![written("input"), int(*c)],
                    Layer::Input { c, h, w, .. } => vec![
                        written("input"),
                        Expr::list(vec![int(*h), int(*w), int(*c)]),
                    ],
                    Layer::Conv {
                        w,
                        b,
                        filters,
                        c,
                        kh,
                        kw,
                        stride,
                        pad,
                    } => vec![
                        written("conv"),
                        nest(w, &[*filters, *c, *kh, *kw]),
                        Expr::list(b.iter().map(shown).collect()),
                        pair_shown(*stride),
                        pair_shown(*pad),
                    ],
                    Layer::Pool { max, size, stride } => vec![
                        written(if *max { "maxpool" } else { "avgpool" }),
                        pair_shown(*size),
                        pair_shown(*stride),
                    ],
                    Layer::Flatten => vec![written("flatten")],
                    Layer::Full { w, b, out, inp } => vec![
                        written("dense"),
                        nest(w, &[*out, *inp]),
                        Expr::list(b.iter().map(shown).collect()),
                    ],
                    Layer::Unit { f, extra } => {
                        let mut parts = vec![written(f)];
                        parts.extend(extra.as_ref().filter(|_| usual(f).is_some()).map(shown));
                        parts
                    }
                })
            })
            .collect(),
    )
}

/// What a layer gives, and what it must remember to send slopes back.
fn forward(layer: &Layer, x: &Values) -> (Values, Vec<usize>) {
    match layer {
        Layer::Input { .. } => (x.clone(), Vec::new()),
        Layer::Conv {
            w,
            b,
            filters,
            c,
            kh,
            kw,
            stride,
            pad,
        } => {
            let (oh, ow) = (
                (x.h + 2 * pad[0] - kh) / stride[0] + 1,
                (x.w + 2 * pad[1] - kw) / stride[1] + 1,
            );
            let mut v = Vec::with_capacity(filters * oh * ow);
            for f in 0..*filters {
                for i in 0..oh {
                    for j in 0..ow {
                        let mut sum = b[f].clone();
                        for ch in 0..*c {
                            for di in 0..*kh {
                                let Some(row) = (i * stride[0] + di)
                                    .checked_sub(pad[0])
                                    .filter(|r| *r < x.h)
                                else {
                                    continue;
                                };
                                for dj in 0..*kw {
                                    let Some(col) = (j * stride[1] + dj)
                                        .checked_sub(pad[1])
                                        .filter(|q| *q < x.w)
                                    else {
                                        continue;
                                    };
                                    sum +=
                                        fl(&w[((f * c + ch) * kh + di) * kw + dj]
                                            * x.at(ch, row, col));
                                }
                            }
                        }
                        v.push(sum);
                    }
                }
            }
            (
                Values {
                    c: *filters,
                    h: oh,
                    w: ow,
                    flat: false,
                    v,
                },
                Vec::new(),
            )
        }
        Layer::Pool { max, size, stride } => {
            let (oh, ow) = (
                (x.h - size[0]) / stride[0] + 1,
                (x.w - size[1]) / stride[1] + 1,
            );
            let mut v = Vec::with_capacity(x.c * oh * ow);
            let mut places = Vec::new();
            for ch in 0..x.c {
                for i in 0..oh {
                    for j in 0..ow {
                        let mut best: Option<(Float, usize)> = None;
                        let mut sum = zero();
                        for di in 0..size[0] {
                            for dj in 0..size[1] {
                                let at = (ch * x.h + i * stride[0] + di) * x.w + j * stride[1] + dj;
                                let here = &x.v[at];
                                if *max {
                                    if best.as_ref().is_none_or(|(b, _)| here > b) {
                                        best = Some((here.clone(), at));
                                    }
                                } else {
                                    sum += here;
                                }
                            }
                        }
                        match best {
                            Some((b, at)) => {
                                v.push(b);
                                places.push(at);
                            }
                            None => v.push(sum / (size[0] * size[1]) as u64),
                        }
                    }
                }
            }
            (
                Values {
                    c: x.c,
                    h: oh,
                    w: ow,
                    flat: false,
                    v,
                },
                places,
            )
        }
        Layer::Flatten => (
            Values {
                c: x.v.len(),
                h: 1,
                w: 1,
                flat: true,
                v: x.v.clone(),
            },
            Vec::new(),
        ),
        Layer::Full { w, b, out, inp } => {
            let v = (0..*out)
                .map(|o| (0..*inp).fold(b[o].clone(), |sum, k| sum + fl(&w[o * inp + k] * &x.v[k])))
                .collect();
            (
                Values {
                    c: *out,
                    h: 1,
                    w: 1,
                    flat: true,
                    v,
                },
                Vec::new(),
            )
        }
        Layer::Unit { f: "softmax", .. } => (x.like(soft(&x.v)), Vec::new()),
        Layer::Unit { f, extra } => (
            x.like(x.v.iter().map(|z| value(f, z, extra.as_ref())).collect()),
            Vec::new(),
        ),
    }
}

/// The slopes by what came in from the slopes by what went out; the slopes by the weights
/// and biases of the layer are added to `grad`.
fn backward(
    layer: &Layer,
    x: &Values,
    y: &Values,
    places: &[usize],
    back: &[Float],
    grad: &mut [Float],
) -> Vec<Float> {
    match layer {
        Layer::Input { .. } | Layer::Flatten => back.to_vec(),
        Layer::Conv {
            w,
            filters,
            c,
            kh,
            kw,
            stride,
            pad,
            ..
        } => {
            let mut down = vec![zero(); x.v.len()];
            let weights = w.len();
            for f in 0..*filters {
                for i in 0..y.h {
                    for j in 0..y.w {
                        let g = &back[(f * y.h + i) * y.w + j];
                        if g.is_zero() {
                            continue;
                        }
                        grad[weights + f] += g;
                        for ch in 0..*c {
                            for di in 0..*kh {
                                let Some(row) = (i * stride[0] + di)
                                    .checked_sub(pad[0])
                                    .filter(|r| *r < x.h)
                                else {
                                    continue;
                                };
                                for dj in 0..*kw {
                                    let Some(col) = (j * stride[1] + dj)
                                        .checked_sub(pad[1])
                                        .filter(|q| *q < x.w)
                                    else {
                                        continue;
                                    };
                                    let at = ((f * c + ch) * kh + di) * kw + dj;
                                    let place = (ch * x.h + row) * x.w + col;
                                    grad[at] += fl(g * &x.v[place]);
                                    down[place] += fl(g * &w[at]);
                                }
                            }
                        }
                    }
                }
            }
            down
        }
        Layer::Pool { max, size, stride } => {
            let mut down = vec![zero(); x.v.len()];
            let mut k = 0;
            for ch in 0..x.c {
                for i in 0..y.h {
                    for j in 0..y.w {
                        match max {
                            true => down[places[k]] += &back[k],
                            false => {
                                let share = fl(&back[k] / (size[0] * size[1]) as u64);
                                for di in 0..size[0] {
                                    for dj in 0..size[1] {
                                        down[(ch * x.h + i * stride[0] + di) * x.w
                                            + j * stride[1]
                                            + dj] += &share;
                                    }
                                }
                            }
                        }
                        k += 1;
                    }
                }
            }
            down
        }
        Layer::Full { w, out, inp, .. } => {
            let mut down = vec![zero(); *inp];
            for o in 0..*out {
                let g = &back[o];
                grad[w.len() + o] += g;
                for k in 0..*inp {
                    grad[o * inp + k] += fl(g * &x.v[k]);
                    down[k] += fl(g * &w[o * inp + k]);
                }
            }
            down
        }
        Layer::Unit { f: "softmax", .. } => {
            let mixed =
                y.v.iter()
                    .zip(back)
                    .fold(zero(), |sum, (a, g)| sum + fl(a * g));
            y.v.iter()
                .zip(back)
                .map(|(a, g)| fl(a * fl(g - &mixed)))
                .collect()
        }
        Layer::Unit { f, extra } => {
            x.v.iter()
                .zip(&y.v)
                .zip(back)
                .map(|((z, a), g)| fl(g * slope(f, z, a, extra.as_ref()).unwrap_or_else(zero)))
                .collect()
        }
    }
}

fn weights_of(layer: &Layer) -> usize {
    match layer {
        Layer::Conv { w, b, .. } | Layer::Full { w, b, .. } => w.len() + b.len(),
        _ => 0,
    }
}

fn run(net: &[Layer], x: &Values) -> Values {
    net.iter()
        .fold(x.clone(), |now, layer| forward(layer, &now).0)
}

/// The loss of one observation and its slopes by every weight and bias, layer by layer.
fn slopes_of(
    net: &[Layer],
    x: &Values,
    t: &[Float],
    entropy: bool,
    grads: &mut [Vec<Float>],
    who: &str,
) -> Result<Float, String> {
    let mut seen = vec![x.clone()];
    let mut kept = Vec::with_capacity(net.len());
    for layer in net {
        let (y, places) = forward(layer, seen.last().expect("values"));
        seen.push(y);
        kept.push(places);
    }
    let y = seen.last().expect("values");
    if y.v.len() != t.len() {
        return Err(format!(
            "{who}: the network gives {} values, a target has {}",
            y.v.len(),
            t.len()
        ));
    }
    let mut loss = zero();
    let mut back: Vec<Float> = Vec::with_capacity(t.len());
    for (a, want) in y.v.iter().zip(t) {
        match entropy {
            true => {
                if want.is_zero() {
                    back.push(zero());
                    continue;
                }
                if *a <= 0 {
                    return Err(format!(
                        "{who}: the entropy needs chances above 0: a last layer of \"softmax\""
                    ));
                }
                loss -= fl(want * a.clone().ln());
                back.push(-fl(want / a));
            }
            false => {
                let miss = fl(a - want);
                loss += fl(miss.square_ref()) / t.len() as u64;
                back.push(miss * 2u32 / t.len() as u64);
            }
        }
    }
    for k in (0..net.len()).rev() {
        back = backward(
            &net[k],
            &seen[k],
            &seen[k + 1],
            &kept[k],
            &back,
            &mut grads[k],
        );
    }
    Ok(loss)
}

fn empty_grads(net: &[Layer]) -> Vec<Vec<Float>> {
    net.iter().map(|l| vec![zero(); weights_of(l)]).collect()
}

/// Every weight and bias of a network in one row, and back.
fn flat_of(net: &[Layer]) -> Vec<Float> {
    net.iter()
        .flat_map(|l| match l {
            Layer::Conv { w, b, .. } | Layer::Full { w, b, .. } => {
                w.iter().chain(b).cloned().collect::<Vec<_>>()
            }
            _ => Vec::new(),
        })
        .collect()
}

fn put(net: &mut [Layer], theta: &[Float]) {
    let mut at = 0;
    for layer in net {
        if let Layer::Conv { w, b, .. } | Layer::Full { w, b, .. } = layer {
            for x in w.iter_mut().chain(b.iter_mut()) {
                *x = theta[at].clone();
                at += 1;
            }
        }
    }
}

/// The places of the weights (not the biases) in the row of all of them.
fn weight_places(net: &[Layer]) -> Vec<bool> {
    net.iter()
        .flat_map(|l| match l {
            Layer::Conv { w, b, .. } | Layer::Full { w, b, .. } => {
                std::iter::repeat_n(true, w.len())
                    .chain(std::iter::repeat_n(false, b.len()))
                    .collect::<Vec<_>>()
            }
            _ => Vec::new(),
        })
        .collect()
}

/// The mean loss over some observations with its slopes, and the part of the weights.
fn batch(
    net: &[Layer],
    xs: &[Values],
    ts: &[Vec<Float>],
    which: &[usize],
    entropy: bool,
    l2: &Float,
    who: &str,
) -> Result<(Float, Vec<Float>), String> {
    let mut grads = empty_grads(net);
    let mut loss = zero();
    for k in which {
        loss += slopes_of(net, &xs[*k], &ts[*k], entropy, &mut grads, who)?;
    }
    let count = which.len() as u64;
    let mut slopes: Vec<Float> = grads.into_iter().flatten().map(|g| g / count).collect();
    loss /= count;
    if !l2.is_zero() {
        let theta = flat_of(net);
        for ((g, w), is_weight) in slopes.iter_mut().zip(&theta).zip(weight_places(net)) {
            if is_weight {
                loss += fl(l2 * fl(w.square_ref())) / 2u32;
                *g += fl(l2 * w);
            }
        }
    }
    Ok((loss, slopes))
}

struct Plan {
    solver: String,
    epochs: usize,
    size: usize,
    rate: Float,
    momentum: Float,
    decay: Float,
    square_decay: Float,
    epsilon: Float,
    l2: Float,
    shuffle: String,
    seed: u64,
    report: bool,
    entropy: Option<bool>,
}

/// The choices of a training by steps: "solver", "adam", "epochs", 20, ...
fn plan(rest: &[Expr], who: &str) -> Result<Plan, String> {
    // (what trainingoptions gives is a list of the same choices)
    let mut all: Vec<Expr> = Vec::new();
    for e in rest {
        match e.as_list() {
            Some(items) if items.first().and_then(text).as_deref() == Some("solver") => {
                all.extend(items.iter().cloned())
            }
            _ => all.push(e.clone()),
        }
    }
    let mut p = Plan {
        solver: "adam".into(),
        epochs: 30,
        size: 128,
        rate: zero(),
        momentum: fl(9) / 10u32,
        decay: fl(9) / 10u32,
        square_decay: zero(),
        epsilon: fl(1) / 100_000_000u32,
        l2: fl(1) / 10_000u32,
        shuffle: "once".into(),
        seed: 1,
        report: false,
        entropy: None,
    };
    let (mut rate, mut square) = (None, None);
    let mut k = 0;
    let usage = || {
        format!(
            "{who}: the choices of a training by steps: \"solver\", \"sgdm\" | \"adam\" | \"rmsprop\"; \"epochs\", n; \"batch\", n; \"rate\", x; \"momentum\", x; \"l2\", x; \"shuffle\", \"once\" | \"never\" | \"every-epoch\"; \"seed\", n; \"mse\" | \"crossentropy\"; \"report\""
        )
    };
    while k < all.len() {
        let name = text(&all[k]).ok_or_else(usage)?;
        match name.as_str() {
            "mse" => p.entropy = Some(false),
            "crossentropy" => p.entropy = Some(true),
            "report" => p.report = true,
            _ => {
                let v = all
                    .get(k + 1)
                    .ok_or_else(|| format!("{who}: a value must follow \"{name}\""))?;
                match name.as_str() {
                    "solver" => {
                        p.solver = text(v)
                            .filter(|s| matches!(s.as_str(), "sgdm" | "adam" | "rmsprop"))
                            .ok_or_else(usage)?;
                    }
                    "epochs" | "maxepochs" => {
                        p.epochs = whole(v, "the number of epochs", 0, 1_000_000)? as usize
                    }
                    "batch" | "minibatchsize" | "batchsize" => {
                        p.size = whole(v, "the size of a batch", 1, 1 << 30)? as usize
                    }
                    "rate" | "learnrate" | "initiallearnrate" => rate = Some(real(v, who)?),
                    "momentum" => p.momentum = real(v, who)?,
                    "gradientdecayfactor" | "beta1" => p.decay = real(v, who)?,
                    "squaredgradientdecayfactor" | "beta2" | "decay" => {
                        square = Some(real(v, who)?)
                    }
                    "epsilon" => p.epsilon = real(v, who)?,
                    "l2" | "l2regularization" => p.l2 = real(v, who)?,
                    "shuffle" => {
                        p.shuffle = text(v)
                            .filter(|s| matches!(s.as_str(), "once" | "never" | "everyepoch"))
                            .ok_or_else(usage)?;
                    }
                    "seed" => p.seed = whole(v, "the seed", 0, i64::MAX)? as u64,
                    _ => return Err(usage()),
                }
                k += 1;
            }
        }
        k += 1;
    }
    p.rate = rate.unwrap_or_else(|| match p.solver.as_str() {
        "sgdm" => fl(1) / 100u32,
        _ => fl(1) / 1000u32,
    });
    p.square_decay = square.unwrap_or_else(|| match p.solver.as_str() {
        "rmsprop" => fl(9) / 10u32,
        _ => fl(999) / 1000u32,
    });
    if p.rate <= 0 || p.momentum < 0 || p.l2 < 0 {
        return Err(usage());
    }
    Ok(p)
}

/// Steps down the slope, a batch of observations at a time; the loss after every epoch.
fn train(
    net: &mut [Layer],
    xs: &[Values],
    ts: &[Vec<Float>],
    entropy: bool,
    p: &Plan,
    who: &str,
) -> Result<Vec<Float>, String> {
    let mut theta = flat_of(net);
    let mut first = vec![zero(); theta.len()];
    let mut second = vec![zero(); theta.len()];
    let mut order: Vec<usize> = (0..xs.len()).collect();
    let mut dice = Shuffle(
        p.seed
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .wrapping_add(0x5851_F42D_4C95_7F2D),
    );
    let mut mix = |order: &mut Vec<usize>| {
        for k in (1..order.len()).rev() {
            let share = (dice.next() + 1.0) / 2.0;
            let other = ((share * (k + 1) as f64) as usize).min(k);
            order.swap(k, other);
        }
    };
    if p.shuffle == "once" {
        mix(&mut order);
    }
    let mut losses = Vec::with_capacity(p.epochs);
    let mut step = 0u32;
    let (mut pow1, mut pow2) = (fl(1), fl(1));
    for _ in 0..p.epochs {
        if p.shuffle == "everyepoch" {
            mix(&mut order);
        }
        for part in order.chunks(p.size) {
            let (_, g) = batch(net, xs, ts, part, entropy, &p.l2, who)?;
            step += 1;
            match p.solver.as_str() {
                "sgdm" => {
                    for ((w, v), g) in theta.iter_mut().zip(first.iter_mut()).zip(&g) {
                        *v = fl(&p.momentum * &*v) - fl(&p.rate * g);
                        *w += &*v;
                    }
                }
                "adam" => {
                    pow1 *= &p.decay;
                    pow2 *= &p.square_decay;
                    for (((w, m), v), g) in theta
                        .iter_mut()
                        .zip(first.iter_mut())
                        .zip(second.iter_mut())
                        .zip(&g)
                    {
                        *m = fl(&p.decay * &*m) + fl(fl(1) - &p.decay) * g;
                        *v = fl(&p.square_decay * &*v)
                            + fl(fl(1) - &p.square_decay) * fl(g.square_ref());
                        let mean = fl(&*m / fl(fl(1) - &pow1));
                        let spread = fl(&*v / fl(fl(1) - &pow2));
                        *w -= fl(&p.rate * mean) / (spread.sqrt() + &p.epsilon);
                    }
                }
                _ => {
                    for ((w, v), g) in theta.iter_mut().zip(second.iter_mut()).zip(&g) {
                        *v = fl(&p.square_decay * &*v)
                            + fl(fl(1) - &p.square_decay) * fl(g.square_ref());
                        *w -= fl(&p.rate * g) / (v.clone().sqrt() + &p.epsilon);
                    }
                }
            }
            put(net, &theta);
        }
        let all: Vec<usize> = (0..xs.len()).collect();
        losses.push(batch(net, xs, ts, &all, entropy, &p.l2, who)?.0);
    }
    let _ = step;
    Ok(losses)
}

/// The observations as values of the size that the network takes: one, or a list of them.
fn observations(e: &Expr, first: &Layer, who: &str) -> Result<(Vec<Values>, bool), String> {
    let Layer::Input { c, h, w, flat } = first else {
        return Err(format!("{who}: {USAGE}"));
    };
    let (c, h, w, flat) = (*c, *h, *w, *flat);
    let one = |e: &Expr| -> Option<Values> {
        let dims = sizes(e);
        let fits = match flat {
            true => dims == [c],
            false => dims == [c, h, w] || (c == 1 && dims == [h, w]),
        };
        let count = if flat { c } else { c * h * w };
        fits.then(|| numbers(e, who).ok())
            .flatten()
            .filter(|v| v.len() == count)
            .map(|v| Values { c, h, w, flat, v })
    };
    if let Some(x) = one(e) {
        return Ok((vec![x], true));
    }
    let wrong = || match flat {
        true => format!(
            "{who}: an observation is a list of {c} values; several of them are the rows of a matrix"
        ),
        false => format!(
            "{who}: an observation is a matrix of {h} by {w} values (a list of {c} such matrices for {c} channels); several of them are a list of those"
        ),
    };
    let items = e.as_list().ok_or_else(wrong)?;
    let all: Vec<Values> = items
        .iter()
        .map(one)
        .collect::<Option<_>>()
        .ok_or_else(wrong)?;
    if all.is_empty() {
        return Err(wrong());
    }
    Ok((all, false))
}

fn value_shown(y: &Values) -> Expr {
    match (y.flat, y.c) {
        (true, _) => Expr::list(y.v.iter().map(shown).collect()),
        (false, 1) => nest(&y.v, &[y.h, y.w]),
        (false, _) => nest(&y.v, &[y.c, y.h, y.w]),
    }
}

/// The targets of observations: lists of values, or the numbers of classes for a network
/// whose last layer is "softmax".
fn targets(
    e: &Expr,
    outs: usize,
    count: usize,
    classes: bool,
    single: bool,
    who: &str,
) -> Result<Vec<Vec<Float>>, String> {
    let items: Vec<Expr> = match (single, e.as_list()) {
        (true, _) => vec![e.clone()],
        (false, Some(items)) => items.to_vec(),
        (false, None) => return Err(format!("{who}: a target for every observation")),
    };
    if items.len() != count {
        return Err(format!(
            "{who}: {count} observations but {} targets",
            items.len()
        ));
    }
    items
        .iter()
        .map(|t| match (classes && t.as_list().is_none(), t) {
            (true, k) => {
                let class = whole(k, "the class of an observation", 1, outs as i64)? as usize;
                Ok((1..=outs).map(|c| fl(i32::from(c == class))).collect())
            }
            (false, t) => {
                let v = numbers(t, who)?;
                match v.len() == outs {
                    true => Ok(v),
                    false => Err(format!(
                        "{who}: the network gives {outs} values, a target has {}",
                        v.len()
                    )),
                }
            }
        })
        .collect()
}

/// A network of dense layers as layers of this kind, and back.
fn from_dense(net: &[Dense]) -> Vec<Layer> {
    let mut out = vec![Layer::Input {
        c: net[0].w[0].len(),
        h: 1,
        w: 1,
        flat: true,
    }];
    for l in net {
        out.push(Layer::Full {
            w: l.w.concat(),
            b: l.b.clone(),
            out: l.w.len(),
            inp: l.w[0].len(),
        });
        out.push(Layer::Unit {
            f: l.f,
            extra: l.extra.clone(),
        });
    }
    out
}

fn to_dense(net: &[Layer]) -> Vec<Dense> {
    let mut out = Vec::new();
    for pair in net[1..].chunks(2) {
        if let [Layer::Full { w, b, inp, .. }, Layer::Unit { f, extra }] = pair {
            out.push(Dense {
                w: w.chunks(*inp).map(<[Float]>::to_vec).collect(),
                b: b.clone(),
                f,
                extra: extra.clone(),
            });
        }
    }
    out
}

fn ends_with_softmax(net: &[Layer]) -> bool {
    matches!(net.last(), Some(Layer::Unit { f: "softmax", .. }))
}

/// The layers as they are asked for, before they have weights.
fn spec(name: &str, a: &[Expr]) -> Res {
    let n = a.len();
    let named = |from: usize, known: &[&str]| -> Result<Vec<(String, Expr)>, String> {
        let mut out = Vec::new();
        let mut k = from;
        while k < n {
            let key = text(&a[k])
                .filter(|w| known.contains(&w.as_str()))
                .ok_or_else(|| {
                    format!(
                        "{name}: {} is not a choice of this layer: {}",
                        a[k],
                        known
                            .iter()
                            .map(|w| format!("\"{w}\""))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })?;
            let v = a
                .get(k + 1)
                .ok_or_else(|| format!("{name}: a value must follow \"{key}\""))?;
            out.push((key, v.clone()));
            k += 2;
        }
        Ok(out)
    };
    Ok(Expr::list(match name {
        "imageinputlayer" => {
            let dims = a[0]
                .as_list()
                .map(<[Expr]>::to_vec)
                .unwrap_or_else(|| a.to_vec());
            let count = |e: &Expr| whole(e, "a size of the pictures", 1, 1 << 16);
            match &dims[..] {
                [h, w] => vec![
                    written("input"),
                    Expr::list(vec![
                        Expr::int(count(h)?),
                        Expr::int(count(w)?),
                        Expr::int(1),
                    ]),
                ],
                [h, w, c] => vec![
                    written("input"),
                    Expr::list(vec![
                        Expr::int(count(h)?),
                        Expr::int(count(w)?),
                        Expr::int(count(c)?),
                    ]),
                ],
                _ => return Err("imageinputlayer([height, width, channels])".into()),
            }
        }
        "featureinputlayer" => vec![
            written("input"),
            Expr::int(whole(&a[0], "the number of values", 1, 1 << 20)?),
        ],
        "convolution2dlayer" => {
            let size = pair(&a[0], "the size of the window", 1)?;
            let filters = whole(a.get(1).ok_or("convolution2dlayer(size of the window, filters[, \"stride\", s][, \"padding\", p])")?, "the number of filters", 1, 4096)?;
            let (mut stride, mut pad) = ([1, 1], Expr::list(vec![Expr::zero(), Expr::zero()]));
            for (key, v) in named(2, &["stride", "padding"])? {
                match key.as_str() {
                    "stride" => stride = pair(&v, "the stride", 1)?,
                    _ => {
                        pad = match text(&v).as_deref() {
                            Some("same") => {
                                if size[0] % 2 == 0 || size[1] % 2 == 0 {
                                    return Err("convolution2dlayer: \"same\" keeps the size for windows of odd sizes".into());
                                }
                                pair_shown([(size[0] - 1) / 2, (size[1] - 1) / 2])
                            }
                            _ => pair_shown(pair(&v, "the padding", 0)?),
                        }
                    }
                }
            }
            vec![
                written("conv"),
                pair_shown(size),
                Expr::int(filters),
                pair_shown(stride),
                pad,
            ]
        }
        "maxpooling2dlayer" | "averagepooling2dlayer" => {
            let size = pair(&a[0], "the size of the window", 1)?;
            let mut stride = [1, 1];
            for (_, v) in named(1, &["stride"])? {
                stride = pair(&v, "the stride", 1)?;
            }
            vec![
                written(if name.starts_with("max") {
                    "maxpool"
                } else {
                    "avgpool"
                }),
                pair_shown(size),
                pair_shown(stride),
            ]
        }
        "flattenlayer" => vec![written("flatten")],
        "fullyconnectedlayer" => vec![
            written("dense"),
            Expr::int(whole(&a[0], "the number of units", 1, 1 << 20)?),
        ],
        "relulayer" => vec![written("relu")],
        "tanhlayer" => vec![written("tanh")],
        "sigmoidlayer" => vec![written("sigmoid")],
        "softmaxlayer" => vec![written("softmax")],
        "leakyrelulayer" => {
            let mut parts = vec![written("leakyrelu")];
            parts.extend(a.first().cloned());
            parts
        }
        _ => return Err(format!("{name} is no layer")),
    }))
}

/// The layers with their first weights: of Glorot, from numbers that are the same on
/// every machine.
fn initialized(layers: &Expr, seed: u64, who: &str) -> Res {
    let items = layers.as_list().ok_or_else(|| {
        format!("{who}([imageinputlayer([height, width, channels]), layers ...][, seed])")
    })?;
    let mut dice = Shuffle(
        seed.wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .wrapping_add(0x1234_5678_9ABC_DEF1),
    );
    // a fraction between -1 and 1, the same on every machine
    let mut share = move || -> Float {
        let k = (dice.next() * (1u64 << 30) as f64).round();
        fl(k) / (1u64 << 30)
    };
    let mut shape: Option<(usize, usize, usize, bool)> = None;
    let mut out = Vec::new();
    for item in items {
        let parts = item
            .as_list()
            .ok_or_else(|| format!("{who}: {item} is no layer"))?;
        let kind = parts
            .first()
            .and_then(text)
            .ok_or_else(|| format!("{who}: {item} is no layer"))?;
        let unsized_conv = kind == "conv" && parts.len() == 5 && sizes(&parts[1]).len() == 1;
        let unsized_dense = kind == "dense" && parts.len() == 2;
        let made = match (unsized_conv, unsized_dense, shape) {
            (true, _, Some((c, _, _, false))) => {
                let size = pair(&parts[1], "the size of the window", 1)?;
                let filters = whole(&parts[2], "the number of filters", 1, 4096)? as usize;
                let limit = (fl(6) / ((size[0] * size[1] * (c + filters)) as u64)).sqrt();
                let w: Vec<Float> = (0..filters * c * size[0] * size[1])
                    .map(|_| fl(&limit * share()))
                    .collect();
                Expr::list(vec![
                    written("conv"),
                    nest(&w, &[filters, c, size[0], size[1]]),
                    Expr::list(vec![Expr::zero(); filters]),
                    parts[3].clone(),
                    parts[4].clone(),
                ])
            }
            (_, true, Some((c, 1, 1, true))) => {
                let units = whole(&parts[1], "the number of units", 1, 1 << 20)? as usize;
                let limit = (fl(6) / ((c + units) as u64)).sqrt();
                let w: Vec<Float> = (0..units * c).map(|_| fl(&limit * share())).collect();
                Expr::list(vec![
                    written("dense"),
                    nest(&w, &[units, c]),
                    Expr::list(vec![Expr::zero(); units]),
                ])
            }
            (true, _, _) => {
                return Err(format!(
                    "{who}: a layer of windows takes channels of matrices: it comes after the input of pictures and before [\"flatten\"]"
                ));
            }
            (_, true, _) => {
                return Err(format!(
                    "{who}: a dense layer takes a list of values: flattenlayer() goes before it"
                ));
            }
            _ => item.clone(),
        };
        out.push(made);
        // the sizes that the next layer gets
        let so_far = read_net(&Expr::list(out.clone()), who);
        let net = match so_far {
            Ok(net) => net,
            Err(_) if out.len() == 1 => {
                let Ok(first) = read_net(
                    &Expr::list(vec![out[0].clone(), Expr::list(vec![written("flatten")])]),
                    who,
                ) else {
                    return Err(format!("{who}: {USAGE}"));
                };
                first[..1].to_vec()
            }
            Err(why) => return Err(why),
        };
        let Layer::Input { c, h, w, flat } = &net[0] else {
            return Err(format!("{who}: {USAGE}"));
        };
        let probe = Values {
            c: *c,
            h: *h,
            w: *w,
            flat: *flat,
            v: vec![zero(); if *flat { *c } else { c * h * w }],
        };
        let y = run(&net, &probe);
        shape = Some((y.c, y.h, y.w, y.flat));
    }
    let net = read_net(&Expr::list(out), who)?;
    Ok(show_net(&net))
}

/// The commands of networks of layers of windows, and of training by steps.
pub(crate) fn call(name: &str, a: &[Expr]) -> Option<Res> {
    let n = a.len();
    Some(match name {
        "imageinputlayer"
        | "featureinputlayer"
        | "convolution2dlayer"
        | "maxpooling2dlayer"
        | "averagepooling2dlayer"
        | "fullyconnectedlayer"
        | "leakyrelulayer"
            if n >= 1 =>
        {
            spec(name, a)
        }
        "flattenlayer" | "relulayer" | "tanhlayer" | "sigmoidlayer" | "softmaxlayer"
        | "leakyrelulayer"
            if n == 0 =>
        {
            spec(name, a)
        }
        "dlnetwork" => {
            let seed = match a.get(1) {
                Some(e) => match whole(e, "the seed", 0, i64::MAX) {
                    Ok(s) => s as u64,
                    Err(why) => return Some(Err(why)),
                },
                None => 0,
            };
            match a.first() {
                Some(layers) if n <= 2 => initialized(layers, seed, name),
                _ => Err(
                    "dlnetwork([imageinputlayer([height, width, channels]), layers ...][, seed])"
                        .into(),
                ),
            }
        }
        "trainingoptions" => (|| -> Res {
            let solver = a.first().and_then(text).filter(|s| matches!(s.as_str(), "sgdm" | "adam" | "rmsprop")).ok_or("trainingoptions(\"sgdm\" | \"adam\" | \"rmsprop\"[, \"MaxEpochs\", n][, \"MiniBatchSize\", n][, \"InitialLearnRate\", x][, \"Momentum\", x][, \"L2Regularization\", x][, \"Shuffle\", \"once\" | \"never\" | \"every-epoch\"][, \"seed\", n])")?;
            let mut all = vec![written("solver"), written(&solver)];
            all.extend(a[1..].iter().cloned());
            plan(std::slice::from_ref(&Expr::list(all.clone())), name)?;
            Ok(Expr::list(all))
        })(),
        "predict" | "classify" if n >= 2 && is_convnet(&a[0]) => (|| -> Res {
            let net = read_net(&a[0], name)?;
            let (xs, single) = observations(&a[1], &net[0], name)?;
            let outs: Vec<Values> = xs.iter().map(|x| run(&net, x)).collect();
            let mut shown_all: Vec<Expr> = match name {
                "predict" if n == 2 => outs.iter().map(value_shown).collect(),
                "classify" if n <= 3 => {
                    let classes = a.get(2).and_then(Expr::as_list);
                    let mut found = Vec::new();
                    for y in &outs {
                        let mut best = 0;
                        for k in 1..y.v.len() {
                            if y.v[k] > y.v[best] {
                                best = k;
                            }
                        }
                        found.push(match classes {
                            Some(names) => names.get(best).cloned().ok_or_else(|| {
                                format!(
                                    "classify: {} classes for {} values",
                                    names.len(),
                                    y.v.len()
                                )
                            })?,
                            None => int(best + 1),
                        });
                    }
                    found
                }
                _ => return Err(format!("{name}(network, observations)")),
            };
            Ok(match single {
                true => shown_all.swap_remove(0),
                false => Expr::list(shown_all),
            })
        })(),
        "perform" if (3..=4).contains(&n) && is_convnet(&a[0]) => (|| -> Res {
            let net = read_net(&a[0], name)?;
            let (xs, single) = observations(&a[1], &net[0], name)?;
            let entropy = match a.get(3).and_then(text).as_deref() {
                Some("mse") => false,
                Some("crossentropy") => true,
                None => ends_with_softmax(&net),
                Some(_) => {
                    return Err(
                        "perform(network, observations, targets[, \"mse\" | \"crossentropy\"])"
                            .into(),
                    );
                }
            };
            let outs = run(&net, &xs[0]).v.len();
            let ts = targets(&a[2], outs, xs.len(), entropy, single, name)?;
            let all: Vec<usize> = (0..xs.len()).collect();
            Ok(shown(
                &batch(&net, &xs, &ts, &all, entropy, &zero(), name)?.0,
            ))
        })(),
        "trainnet"
            if n >= 3 && (is_convnet(&a[0]) || (is_net(&a[0]) && a[3..].iter().any(stepped))) =>
        {
            (|| -> Res {
                let p = plan(&a[3..], name)?;
                let plain = !is_convnet(&a[0]);
                let mut net = match plain {
                    true => from_dense(&dense(&net_of(&a[0], name)?, name)?),
                    false => read_net(&a[0], name)?,
                };
                let (xs, single) = observations(&a[1], &net[0], name)?;
                let entropy = p.entropy.unwrap_or_else(|| ends_with_softmax(&net));
                if entropy && !ends_with_softmax(&net) {
                    return Err(format!(
                        "{name}: the entropy goes with a last layer of \"softmax\""
                    ));
                }
                let outs = run(&net, &xs[0]).v.len();
                let ts = targets(&a[2], outs, xs.len(), entropy, single, name)?;
                let losses = train(&mut net, &xs, &ts, entropy, &p, name)?;
                let trained = match plain {
                    true => net_shown(&to_dense(&net)),
                    false => show_net(&net),
                };
                Ok(match p.report {
                    true => Expr::list(vec![
                        trained,
                        Expr::list(losses.iter().map(shown).collect()),
                    ]),
                    false => trained,
                })
            })()
        }
        _ => return None,
    })
}

/// A choice that asks for a training by steps (of a network of dense layers, which
/// otherwise is trained to the least loss).
fn stepped(e: &Expr) -> bool {
    match e.as_list() {
        Some(items) => items.first().and_then(text).as_deref() == Some("solver"),
        None => matches!(
            text(e).as_deref(),
            Some(
                "solver"
                    | "batch"
                    | "minibatchsize"
                    | "batchsize"
                    | "rate"
                    | "learnrate"
                    | "initiallearnrate"
                    | "momentum"
                    | "shuffle"
                    | "seed"
            )
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn said(input: &str) -> String {
        crate::eval::run_line(input).map_or_else(|e| format!("error: {e}"), |lines| lines.join("|"))
    }

    fn value_of(input: &str) -> Expr {
        let parsed = crate::parser::parse(input).expect("an expression");
        crate::eval::Session::new()
            .eval(&parsed.stmts[0])
            .expect("a value")
    }

    const NET: &str = "[[\"input\", [4, 4, 1]], [\"conv\", [[[[1, 0], [0, -1]]], [[[1/2, 1/2], [1/2, 1/2]]]], [0, 1], [1, 1], [0, 0]], [\"relu\"], [\"maxpool\", [2, 2], [1, 1]], [\"flatten\"], [\"dense\", [[1, 0, 0, 0, 1, 0, 0, 0], [0, 1, 0, 0, 0, 0, 0, 1]], [0, 1/2]], [\"softmax\"]]";
    const PICTURE: &str = "[[1, 2, 0, 1], [0, 1, 3, 1], [2, 0, 1, 0], [1, 1, 0, 2]]";

    #[test]
    fn a_picture_goes_through_the_layers() {
        // windows, the larger of four, a dense layer
        assert_eq!(
            said(&format!(
                "predict([[\"input\", [4, 4, 1]], [\"conv\", [[[[1, 0], [0, -1]]]], [0], [1, 1], [0, 0]], [\"relu\"], [\"maxpool\", [2, 2], [1, 1]]], {PICTURE})"
            )),
            "[[0, 3], [1, 3]]"
        );
        assert_eq!(said(&format!("classify({NET}, {PICTURE})")), "2");
        assert!(said(&format!("predict({NET}, [[1, 2], [3, 4]])")).contains("4 by 4"));
        assert_eq!(
            said(
                "length(part(part(dlnetwork([imageinputlayer([6, 6, 1]), convolution2dlayer(3, 2), relulayer(), maxpooling2dlayer(2, \"stride\", 2), flattenlayer(), fullyconnectedlayer(3), softmaxlayer()], 7), 6), 2))"
            ),
            "3"
        );
    }

    #[test]
    fn the_slopes_agree_with_differences() {
        // (values that lie on no corner of a function and are no two the same under a window)
        let net = "[[\"input\", [4, 4, 2]], [\"conv\", [[[[1, 1/3], [1/5, -1]], [[1/7, 2], [-1/2, 1/9]]], [[[1/2, -1/3], [1/4, 1/6]], [[-1, 1/8], [1/3, 1/2]]]], [1/10, -1/7], [1, 1], [1, 0]], [\"tanh\"], [\"maxpool\", [2, 2], [1, 1]], [\"conv\", [[[[1/2]], [[-1/3]]]], [1/5], [1, 1], [0, 0]], [\"leakyrelu\", 1/10], [\"avgpool\", [2, 1], [1, 1]], [\"flatten\"], [\"dense\", [[1, -1/2, 1/3, 1/4, -1/5, 1/6], [1/7, 1/8, -1/9, 1, 1/2, -1/3]], [1/20, 1/2]], [\"softmax\"]]";
        let picture = "[[[1/3, 2, 1/7, 1], [1/9, 1, 3, 5/4], [2, 1/11, 1, 2/13], [1, 6/5, 1/17, 2]], [[1/2, -1, 3/7, 2], [4/9, 1/3, -3, 1/4], [-2, 5/11, 1, 7/13], [1/5, -6/5, 3/17, 1]]]";
        let net = read_net(&value_of(net), "test").expect("layers");
        let (xs, _) = observations(&value_of(picture), &net[0], "test").expect("values");
        let t = vec![fl(1) / 4u32, fl(3) / 4u32];
        let mut grads = empty_grads(&net);
        slopes_of(&net, &xs[0], &t, true, &mut grads, "test").expect("slopes");
        let analytic: Vec<Float> = grads.into_iter().flatten().collect();
        let theta = flat_of(&net);
        let tiny = fl(1) >> 40;
        for k in 0..theta.len() {
            let loss_at = |shift: &Float| -> Float {
                let mut moved = theta.clone();
                moved[k] += shift;
                let mut other = net.clone();
                put(&mut other, &moved);
                let mut unused = empty_grads(&other);
                slopes_of(&other, &xs[0], &t, true, &mut unused, "test").expect("a loss")
            };
            let numeric = fl(loss_at(&tiny) - loss_at(&fl(-&tiny))) / fl(&tiny * 2u32);
            let apart = fl(&numeric - &analytic[k]).abs();
            assert!(
                apart < fl(1) >> 60,
                "weight {k}: {numeric} and {}",
                analytic[k]
            );
        }
    }

    #[test]
    fn a_network_of_windows_learns() {
        // two kinds of pictures: a bar that lies and a bar that stands
        let lies = "[[0, 0, 0], [1, 1, 1], [0, 0, 0]]";
        let stands = "[[0, 1, 0], [0, 1, 0], [0, 1, 0]]";
        let net = "dlnetwork([imageinputlayer([3, 3, 1]), convolution2dlayer(2, 2), relulayer(), flattenlayer(), fullyconnectedlayer(2), softmaxlayer()], 3)";
        let taught = format!(
            "trainnet({net}, [{lies}, {stands}], [1, 2], \"solver\", \"adam\", \"rate\", 1/20, \"epochs\", 60)"
        );
        assert_eq!(
            said(&format!("classify({taught}, [{lies}, {stands}])")),
            "[1, 2]"
        );
        // the loss falls from epoch to epoch at the end
        let losses = format!(
            "part({}, 2)",
            taught.replace("\"epochs\", 60)", "\"epochs\", 60, \"report\")")
        );
        assert_eq!(
            said(&format!("sign(part({losses}, 59) - part({losses}, 60))")),
            "1"
        );
    }
}
