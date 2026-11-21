//! Records with named fields (the structs of MATLAB) and tables of keys and values (its
//! dictionaries): both are written, shown and kept as the call that makes them.

use crate::eval::Session;
use crate::expr::Expr;
use crate::radix::text_of;
use crate::texts::text;

type Res = Result<Expr, String>;
type Pairs = Vec<(Expr, Expr)>;

const RECORD: &str = "struct";
const TABLE: &str = "dictionary";

fn pairs(e: &Expr, kind: &str) -> Option<Pairs> {
    let args = e.fn_args(kind)?;
    (args.len() % 2 == 0).then(|| {
        args.chunks(2)
            .map(|c| (c[0].clone(), c[1].clone()))
            .collect()
    })
}

pub(crate) fn made(kind: &str, pairs: Pairs) -> Expr {
    Expr::call(kind, pairs.into_iter().flat_map(|(k, v)| [k, v]).collect())
}

/// A record of names and values, as the programs of this crate make one.
pub(crate) fn record(fields: Vec<(&str, Expr)>) -> Expr {
    made(
        RECORD,
        fields.into_iter().map(|(k, v)| (text(k), v)).collect(),
    )
}

pub(crate) fn is_record(e: &Expr) -> bool {
    pairs(e, RECORD).is_some()
}

pub(crate) fn is_table(e: &Expr) -> bool {
    pairs(e, TABLE).is_some()
}

fn fields(e: &Expr, name: &str) -> Result<Pairs, String> {
    pairs(e, RECORD).ok_or_else(|| {
        format!("{name} needs a record made by struct(name, value, ...), {e} is none")
    })
}

fn entries(e: &Expr, name: &str) -> Result<Pairs, String> {
    pairs(e, TABLE).ok_or_else(|| {
        format!("{name} needs a table made by dictionary(keys, values), {e} is none")
    })
}

fn field_name(e: &Expr, name: &str) -> Result<String, String> {
    let word = text_of(e)
        .ok_or_else(|| format!("{name} needs the name of a field in quotes, {e} is none"))?;
    let mut signs = word.chars();
    let fits = signs.next().is_some_and(char::is_alphabetic)
        && signs.all(|c| c.is_alphanumeric() || c == '_');
    match fits {
        true => Ok(word.to_string()),
        false => Err(format!(
            "{name}: \"{word}\" is no name of a field (letters, digits and _ after a letter)"
        )),
    }
}

/// The names that an argument gives: one in quotes or a list of them.
fn names_of(e: &Expr, name: &str) -> Result<Vec<String>, String> {
    match e.as_list() {
        Some(items) => items.iter().map(|x| field_name(x, name)).collect(),
        None => Ok(vec![field_name(e, name)?]),
    }
}

fn key_of(e: &Expr, name: &str) -> Result<Expr, String> {
    match e.as_list() {
        Some(_) => Err(format!(
            "{name}: a key is a number or a text, {e} is a list"
        )),
        None => Ok(e.clone()),
    }
}

/// The keys that an argument gives: one, or a list of them.
fn keys_of(e: &Expr, name: &str) -> Result<Vec<Expr>, String> {
    match e.as_list() {
        Some(items) => items.iter().map(|x| key_of(x, name)).collect(),
        None => Ok(vec![e.clone()]),
    }
}

fn put(all: &mut Pairs, key: Expr, value: Expr) {
    match all.iter_mut().find(|(k, _)| *k == key) {
        Some(entry) => entry.1 = value,
        None => all.push((key, value)),
    }
}

/// A record with a field set; more than one name goes into the records within.
fn set_field(e: &Expr, path: &[String], value: Expr, name: &str) -> Res {
    let mut all = fields(e, name)?;
    let key = text(&path[0]);
    let new = match &path[1..] {
        [] => value,
        rest => {
            let inner = all
                .iter()
                .find(|(k, _)| *k == key)
                .map_or_else(|| made(RECORD, Vec::new()), |(_, v)| v.clone());
            set_field(&inner, rest, value, name)?
        }
    };
    put(&mut all, key, new);
    Ok(made(RECORD, all))
}

fn get_field(e: &Expr, path: &[String], name: &str) -> Res {
    let all = fields(e, name)?;
    let key = text(&path[0]);
    let found = all
        .into_iter()
        .find(|(k, _)| *k == key)
        .ok_or_else(|| format!("{name}: the record has no field \"{}\"", path[0]))?
        .1;
    match &path[1..] {
        [] => Ok(found),
        rest => get_field(&found, rest, name),
    }
}

/// One for one argument, a list for a list.
fn one_or_all(e: &Expr, all: Vec<Expr>) -> Expr {
    match e.as_list() {
        Some(_) => Expr::list(all),
        None => all.into_iter().next().unwrap_or_else(Expr::undefined),
    }
}

fn inner(name: &str, a: &[Expr]) -> Result<Option<Expr>, String> {
    let n = a.len();
    Ok(Some(match (name, n) {
        ("struct", _) => {
            if !n.is_multiple_of(2) {
                return Err("struct(name, value, ...) needs a value for every name".into());
            }
            let mut all: Pairs = Vec::new();
            for pair in a.chunks(2) {
                let key = text(&field_name(&pair[0], name)?);
                if all.iter().any(|(k, _)| *k == key) {
                    return Err(format!("struct: the field {key} is there twice"));
                }
                all.push((key, pair[1].clone()));
            }
            made(RECORD, all)
        }
        ("isstruct", 1) => Expr::Bool(is_record(&a[0])),
        ("fieldnames", 1) => Expr::list(fields(&a[0], name)?.into_iter().map(|(k, _)| k).collect()),
        ("numfields", 1) => Expr::int(pairs(&a[0], RECORD).map_or(0, |all| all.len() as i64)),
        ("struct2cell", 1) => {
            Expr::list(fields(&a[0], name)?.into_iter().map(|(_, v)| v).collect())
        }
        ("cell2struct", 2) => {
            let usage = "cell2struct([values], [names]) needs a name for every value";
            let (values, names) = (a[0].as_list().ok_or(usage)?, names_of(&a[1], name)?);
            if values.len() != names.len() {
                return Err(usage.into());
            }
            let all: Vec<Expr> = names
                .iter()
                .zip(values)
                .flat_map(|(k, v)| [text(k), v.clone()])
                .collect();
            return inner("struct", &all);
        }
        ("isfield", 2) => {
            let all = pairs(&a[0], RECORD).unwrap_or_default();
            let has = |x: &Expr| Expr::Bool(all.iter().any(|(k, _)| k == x));
            match a[1].as_list() {
                Some(items) => Expr::list(items.iter().map(has).collect()),
                None => has(&a[1]),
            }
        }
        ("getfield", 2..) => {
            let path: Vec<String> = a[1..]
                .iter()
                .map(|x| field_name(x, name))
                .collect::<Result<_, _>>()?;
            get_field(&a[0], &path, name)?
        }
        ("setfield", 3..) => {
            let path: Vec<String> = a[1..n - 1]
                .iter()
                .map(|x| field_name(x, name))
                .collect::<Result<_, _>>()?;
            set_field(&a[0], &path, a[n - 1].clone(), name)?
        }
        ("rmfield", 2) => {
            let mut all = fields(&a[0], name)?;
            for gone in names_of(&a[1], name)? {
                let key = text(&gone);
                let at = all
                    .iter()
                    .position(|(k, _)| *k == key)
                    .ok_or_else(|| format!("rmfield: the record has no field \"{gone}\""))?;
                all.remove(at);
            }
            made(RECORD, all)
        }
        ("orderfields", 1) => {
            let mut all = fields(&a[0], name)?;
            all.sort_by(|p, q| text_of(&p.0).cmp(&text_of(&q.0)));
            made(RECORD, all)
        }
        ("dictionary", _) => {
            let mut all: Pairs = Vec::new();
            match (
                a.first().and_then(Expr::as_list),
                a.get(1).and_then(Expr::as_list),
            ) {
                (Some(keys), Some(values)) if n == 2 => {
                    if keys.len() != values.len() {
                        return Err("dictionary(keys, values) needs a value for every key".into());
                    }
                    for (k, v) in keys.iter().zip(values) {
                        put(&mut all, key_of(k, name)?, v.clone());
                    }
                }
                _ if n.is_multiple_of(2) => {
                    for pair in a.chunks(2) {
                        put(&mut all, key_of(&pair[0], name)?, pair[1].clone());
                    }
                }
                _ => {
                    return Err(
                        "dictionary(keys, values) or dictionary(key, value, ...): a value for every key"
                            .into(),
                    );
                }
            }
            made(TABLE, all)
        }
        ("keys", 1) => Expr::list(entries(&a[0], name)?.into_iter().map(|(k, _)| k).collect()),
        ("values", 1) => Expr::list(entries(&a[0], name)?.into_iter().map(|(_, v)| v).collect()),
        ("entries", 1) => Expr::list(
            entries(&a[0], name)?
                .into_iter()
                .map(|(k, v)| Expr::list(vec![k, v]))
                .collect(),
        ),
        ("numentries", 1) => Expr::int(entries(&a[0], name)?.len() as i64),
        ("iskey", 2) => {
            let all = entries(&a[0], name)?;
            let found: Vec<Expr> = keys_of(&a[1], name)?
                .iter()
                .map(|key| Expr::Bool(all.iter().any(|(k, _)| k == key)))
                .collect();
            one_or_all(&a[1], found)
        }
        ("values" | "lookup", 2 | 3) if !(name == "values" && n == 3) => {
            let all = entries(&a[0], name)?;
            let found: Vec<Expr> = keys_of(&a[1], name)?
                .iter()
                .map(|key| match (all.iter().find(|(k, _)| k == key), a.get(2)) {
                    (Some((_, v)), _) => Ok(v.clone()),
                    (None, Some(fallback)) => Ok(fallback.clone()),
                    (None, None) => Err(format!("{name}: the table has no key {key}")),
                })
                .collect::<Result<_, _>>()?;
            one_or_all(&a[1], found)
        }
        ("insert", 3) if is_table(&a[0]) => {
            let mut all = entries(&a[0], name)?;
            match (a[1].as_list(), a[2].as_list()) {
                (Some(keys), Some(values)) if keys.len() == values.len() => {
                    for (k, v) in keys.iter().zip(values) {
                        put(&mut all, key_of(k, name)?, v.clone());
                    }
                }
                (Some(_), _) => {
                    return Err(
                        "insert(table, [keys], [values]) needs a value for every key".into(),
                    );
                }
                (None, _) => put(&mut all, a[1].clone(), a[2].clone()),
            }
            made(TABLE, all)
        }
        ("remove", 2) => {
            let mut all = entries(&a[0], name)?;
            for key in keys_of(&a[1], name)? {
                let at = all
                    .iter()
                    .position(|(k, _)| *k == key)
                    .ok_or_else(|| format!("remove: the table has no key {key}"))?;
                all.remove(at);
            }
            made(TABLE, all)
        }
        ("iscell", 1) => {
            let kind = crate::extras::call("class", a).ok_or("no kinds of values")??;
            Expr::Bool(text_of(&kind) == Some("cell"))
        }
        ("iscellstr", 1) => Expr::Bool(
            a[0].as_list()
                .is_some_and(|items| items.iter().all(|x| text_of(x).is_some())),
        ),
        _ => return Ok(None),
    }))
}

/// A function of records by its name, as it was before cell arrays wrapped its answer.
pub(crate) fn call_plain(name: &str, a: &[Expr]) -> Res {
    inner(name, a)?.ok_or_else(|| format!("{name}: no such function of records"))
}

impl Session {
    pub(crate) fn record_call(&mut self, name: &str, a: &[Expr]) -> Option<Res> {
        if name == "structfun" && a.len() == 2 {
            return Some((|| {
                let all = fields(&a[1], name)?;
                let mut out = Vec::new();
                for (_, value) in all {
                    out.push(self.apply(&a[0], &[value])?);
                }
                Ok(Expr::list(out))
            })());
        }
        inner(name, a).transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_tables_keep_their_order() {
        let s = inner(
            "struct",
            &[text("b"), Expr::int(1), text("a"), Expr::int(2)],
        )
        .expect("a record")
        .expect("a value");
        assert!(is_record(&s) && !is_table(&s));
        let names = inner("fieldnames", std::slice::from_ref(&s));
        assert_eq!(names, Ok(Some(Expr::list(vec![text("b"), text("a")]))));
        let path = ["c".to_string(), "d".to_string()];
        let deep = set_field(&s, &path, Expr::int(3), "setfield").expect("a record");
        assert_eq!(get_field(&deep, &path, "getfield"), Ok(Expr::int(3)));
        assert!(get_field(&deep, &["x".to_string()], "getfield").is_err());
        // a key that is set again keeps its place
        let d = inner(
            "dictionary",
            &[
                Expr::int(1),
                text("x"),
                Expr::int(2),
                text("y"),
                Expr::int(1),
                text("z"),
            ],
        );
        let d = d.expect("a table").expect("a value");
        assert_eq!(
            pairs(&d, TABLE),
            Some(vec![(Expr::int(1), text("z")), (Expr::int(2), text("y"))])
        );
        // made again from what it shows, a table is the same
        let again = inner("dictionary", d.fn_args(TABLE).expect("arguments"));
        assert_eq!(again, Ok(Some(d)));
        assert!(inner("struct", &[text("1a"), Expr::int(1)]).is_err());
        assert!(
            inner(
                "struct",
                &[text("a"), Expr::int(1), text("a"), Expr::int(2)]
            )
            .is_err()
        );
    }
}
