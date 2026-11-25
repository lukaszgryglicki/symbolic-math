//! The calendar: days counted as MATLAB counts them (day 1 is the first of January of the
//! year 0, in the calendar of today carried back), the parts of a date and their names.

use crate::arrays::whole;
use crate::expr::Expr;
use crate::num::{Num, Rat};
use crate::radix::text_of;
use crate::texts::text;
use rug::Integer;

type Res = Result<Expr, String>;

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const DAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// The forms of a date that MATLAB numbers from 0 to 31.
const FORMS: [&str; 32] = [
    "dd-mmm-yyyy HH:MM:SS",
    "dd-mmm-yyyy",
    "mm/dd/yy",
    "mmm",
    "m",
    "mm",
    "mm/dd",
    "dd",
    "ddd",
    "d",
    "yyyy",
    "yy",
    "mmmyy",
    "HH:MM:SS",
    "HH:MM:SS PM",
    "HH:MM",
    "HH:MM PM",
    "QQ-YY",
    "QQ",
    "dd/mm",
    "dd/mm/yy",
    "mmm.dd,yyyy HH:MM:SS",
    "mmm.dd,yyyy",
    "mm/dd/yyyy",
    "dd/mm/yyyy",
    "yy/mm/dd",
    "yyyy/mm/dd",
    "QQ-YYYY",
    "mmmyyyy",
    "yyyy-mm-dd",
    "yyyymmddTHHMMSS",
    "yyyy-mm-dd HH:MM:SS",
];

/// The forms with which a date is read when none is given, in this order.
const READ: [&str; 22] = [
    "yyyy-mm-dd HH:MM:SS.FFF",
    "yyyy-mm-dd HH:MM:SS",
    "yyyy-mm-ddTHH:MM:SS.FFF",
    "yyyy-mm-ddTHH:MM:SS",
    "yyyy-mm-dd HH:MM",
    "yyyy-mm-dd",
    "yyyy/mm/dd HH:MM:SS",
    "yyyy/mm/dd HH:MM",
    "yyyy/mm/dd",
    "dd-mmm-yyyy HH:MM:SS.FFF",
    "dd-mmm-yyyy HH:MM:SS",
    "dd-mmm-yyyy HH:MM",
    "dd-mmm-yyyy",
    "mm/dd/yyyy HH:MM:SS",
    "mm/dd/yyyy HH:MM",
    "mm/dd/yyyy",
    "mmm.dd,yyyy HH:MM:SS",
    "mmm.dd,yyyy",
    "mmm dd, yyyy",
    "mmmm dd, yyyy",
    "dd mmmm yyyy",
    "yyyymmddTHHMMSS",
];

/// The day number of the first of January of 1970.
const EPOCH: i64 = 719_529;
/// The years that the calendar reaches to either side.
const YEARS: i64 = 1_000_000_000;
const MILLIS: i64 = 86_400_000;

#[derive(Clone, Copy, PartialEq)]
enum Piece {
    Year,
    ShortYear,
    Quarter,
    MonthName,
    MonthShort,
    Month,
    MonthLetter,
    DayName,
    DayShort,
    Day,
    DayLetter,
    Hour,
    Minute,
    Second,
    Milli,
    Half,
    Sign(char),
}

/// The pieces of a form: the longest that fits first, any other sign stands for itself.
fn pieces(form: &str) -> Vec<Piece> {
    const WORDS: [(&str, Piece); 19] = [
        ("yyyy", Piece::Year),
        ("YYYY", Piece::Year),
        ("yy", Piece::ShortYear),
        ("YY", Piece::ShortYear),
        ("QQ", Piece::Quarter),
        ("mmmm", Piece::MonthName),
        ("mmm", Piece::MonthShort),
        ("mm", Piece::Month),
        ("m", Piece::MonthLetter),
        ("dddd", Piece::DayName),
        ("ddd", Piece::DayShort),
        ("dd", Piece::Day),
        ("d", Piece::DayLetter),
        ("HH", Piece::Hour),
        ("MM", Piece::Minute),
        ("SS", Piece::Second),
        ("FFF", Piece::Milli),
        ("AM", Piece::Half),
        ("PM", Piece::Half),
    ];
    let mut out = Vec::new();
    let mut rest = form;
    'next: while let Some(first) = rest.chars().next() {
        for (word, piece) in WORDS {
            if let Some(after) = rest.strip_prefix(word) {
                out.push(piece);
                rest = after;
                continue 'next;
            }
        }
        out.push(Piece::Sign(first));
        rest = &rest[first.len_utf8()..];
    }
    out
}

fn number(r: Rat) -> Expr {
    Expr::num(Num::real(r))
}

fn rational(e: &Expr, name: &str) -> Result<Rat, String> {
    if let Some(r) = e.as_rat() {
        return Ok(r.clone());
    }
    let none = || format!("{name} needs real numbers, {e} is none");
    let v = crate::numeric::evalf_digits(e, 40).map_err(|_| none())?;
    match v.imag().is_zero() {
        true => v.real().to_rational().ok_or_else(none),
        false => Err(none()),
    }
}

fn floor(r: &Rat) -> Integer {
    Integer::from(r.floor_ref())
}

/// The number of the day before the first of a month; months beyond 12 are of later years
/// and, as in MATLAB, months before the first are the first.
fn before(year: i64, month: i64) -> i64 {
    let month = month.max(1);
    let (year, month) = (year + (month - 1) / 12, (month - 1) % 12 + 1);
    let y = if month <= 2 { year - 1 } else { year };
    let (era, of_era) = (y.div_euclid(400), y.rem_euclid(400));
    let of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5;
    era * 146_097 + of_era * 365 + of_era / 4 - of_era / 100 + of_year + 60
}

/// The year, the month and the day of the month of a day number.
fn civil(day: i64) -> (i64, i64, i64) {
    let z = day - 61;
    let (era, of_era) = (z.div_euclid(146_097), z.rem_euclid(146_097));
    let year = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year + year / 4 - year / 100);
    let turn = (5 * of_year + 2) / 153;
    let day = of_year - (153 * turn + 2) / 5 + 1;
    let month = if turn < 10 { turn + 3 } else { turn - 9 };
    (year + era * 400 + i64::from(month <= 2), month, day)
}

fn leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn last_day(year: i64, month: i64) -> i64 {
    match month {
        2 => 28 + i64::from(leap(year)),
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// The place of a day in its week, 0 for Sunday.
fn week_place(day: i64) -> usize {
    (day - 2).rem_euclid(7) as usize
}

fn year_of(r: &Rat, what: &str, name: &str) -> Result<i64, String> {
    r.is_integer()
        .then(|| r.numer().to_i64())
        .flatten()
        .filter(|y| y.unsigned_abs() <= YEARS as u64)
        .ok_or_else(|| match r.is_integer() {
            true => format!("{name}: the calendar does not reach that far"),
            false => format!("{name} needs a whole {what}, {r} is none"),
        })
}

/// The day number of [year, month, day] with or without [hour, minute, second].
fn serial(parts: &[Rat], name: &str) -> Result<Rat, String> {
    let year = year_of(&parts[0], "year", name)?;
    let month = year_of(&parts[1], "month", name)?;
    let mut days = Rat::from(before(year, month)) + &parts[2];
    for (part, per) in parts.iter().skip(3).zip([24, 1440, 86_400]) {
        days += Rat::from(part / Rat::from(per));
    }
    Ok(days)
}

/// A day number as the whole day and the thousandths of seconds into it.
fn split(n: &Rat, name: &str) -> Result<(i64, i64), String> {
    let far = || format!("{name}: the calendar does not reach that far");
    let whole = floor(n);
    let rest = Rat::from(n - &whole) * Rat::from(MILLIS);
    let millis = floor(&(rest + Rat::from((1, 2))))
        .to_i64()
        .ok_or_else(far)?;
    let day = whole
        .to_i64()
        .filter(|d| d.unsigned_abs() <= (YEARS * 366) as u64)
        .ok_or_else(far)?;
    Ok(match millis == MILLIS {
        true => (day + 1, 0),
        false => (day, millis),
    })
}

/// A day number in a form of MATLAB.
fn written(n: &Rat, form: &str, name: &str) -> Result<String, String> {
    let (day, millis) = split(n, name)?;
    let (year, month, of_month) = civil(day);
    let (hour, minute) = (millis / 3_600_000, millis / 60_000 % 60);
    let (second, milli) = (millis / 1000 % 60, millis % 1000);
    let (month_name, day_name) = (MONTHS[month as usize - 1], DAYS[week_place(day)]);
    let all = pieces(form);
    let halves = all.contains(&Piece::Half);
    let mut out = String::new();
    for piece in all {
        match piece {
            Piece::Year => out.push_str(&format!("{year:04}")),
            Piece::ShortYear => out.push_str(&format!("{:02}", year.rem_euclid(100))),
            Piece::Quarter => out.push_str(&format!("Q{}", (month - 1) / 3 + 1)),
            Piece::MonthName => out.push_str(month_name),
            Piece::MonthShort => out.push_str(&month_name[..3]),
            Piece::Month => out.push_str(&format!("{month:02}")),
            Piece::MonthLetter => out.push_str(&month_name[..1]),
            Piece::DayName => out.push_str(day_name),
            Piece::DayShort => out.push_str(&day_name[..3]),
            Piece::Day => out.push_str(&format!("{of_month:02}")),
            Piece::DayLetter => out.push_str(&day_name[..1]),
            Piece::Hour if halves => out.push_str(&format!("{:2}", (hour + 11) % 12 + 1)),
            Piece::Hour => out.push_str(&format!("{hour:02}")),
            Piece::Minute => out.push_str(&format!("{minute:02}")),
            Piece::Second => out.push_str(&format!("{second:02}")),
            Piece::Milli => out.push_str(&format!("{milli:03}")),
            Piece::Half => out.push_str(if hour < 12 { "AM" } else { "PM" }),
            Piece::Sign(c) => out.push(c),
        }
    }
    Ok(out)
}

/// The digits at a place of a text, at most so many, as a number and how many they are.
fn digits(text: &[char], at: &mut usize, most: usize) -> Option<(i64, usize)> {
    let from = *at;
    let mut value = 0i64;
    while *at < text.len() && *at - from < most {
        let Some(digit) = text[*at].to_digit(10) else {
            break;
        };
        value = value * 10 + i64::from(digit);
        *at += 1;
    }
    (*at > from).then_some((value, *at - from))
}

/// The name of a table that stands at a place of a text, whole or by its first three letters.
fn named(text: &[char], at: &mut usize, table: &[&str], whole: bool) -> Option<usize> {
    table.iter().position(|name| {
        let name: Vec<char> = match whole {
            true => name.chars().collect(),
            false => name.chars().take(3).collect(),
        };
        let fits = text.len() >= *at + name.len()
            && name
                .iter()
                .zip(&text[*at..])
                .all(|(a, b)| a.eq_ignore_ascii_case(b));
        if fits {
            *at += name.len();
        }
        fits
    })
}

/// The day number of a date written in a form, when it is written so.
fn read_with(s: &str, form: &str) -> Option<Rat> {
    let text: Vec<char> = s.trim().chars().collect();
    let mut at = 0;
    let mut year = None;
    let (mut month, mut day, mut hour, mut minute, mut second, mut milli) = (1, 1, 0, 0, 0, 0);
    let mut late = None;
    for piece in pieces(form) {
        match piece {
            Piece::Year => year = Some(digits(&text, &mut at, 4)?.0),
            Piece::ShortYear => {
                let (y, count) = digits(&text, &mut at, 2)?;
                year = Some(y + if count < 2 || y < 50 { 2000 } else { 1900 });
            }
            Piece::Month => month = digits(&text, &mut at, 2)?.0,
            Piece::MonthName => month = named(&text, &mut at, &MONTHS, true)? as i64 + 1,
            Piece::MonthShort => month = named(&text, &mut at, &MONTHS, false)? as i64 + 1,
            Piece::Day => day = digits(&text, &mut at, 2)?.0,
            Piece::DayName => {
                named(&text, &mut at, &DAYS, true)?;
            }
            Piece::DayShort => {
                named(&text, &mut at, &DAYS, false)?;
            }
            Piece::Hour => hour = digits(&text, &mut at, 2)?.0,
            Piece::Minute => minute = digits(&text, &mut at, 2)?.0,
            Piece::Second => second = digits(&text, &mut at, 2)?.0,
            Piece::Milli => {
                let (value, count) = digits(&text, &mut at, 3)?;
                milli = value * 10i64.pow(3 - count as u32);
            }
            Piece::Half => late = Some(named(&text, &mut at, &["AM", "PM"], true)? == 1),
            Piece::Quarter | Piece::MonthLetter | Piece::DayLetter => return None,
            Piece::Sign(c) => {
                if text.get(at) != Some(&c) {
                    return None;
                }
                at += 1;
            }
        }
    }
    let year = year?;
    if let Some(late) = late {
        if !(1..=12).contains(&hour) {
            return None;
        }
        hour = hour % 12 + if late { 12 } else { 0 };
    }
    let fits = at == text.len()
        && (1..=12).contains(&month)
        && (1..=last_day(year, month)).contains(&day)
        && hour < 24
        && minute < 60
        && second < 60;
    let millis = ((hour * 60 + minute) * 60 + second) * 1000 + milli;
    fits.then(|| Rat::from(before(year, month) + day) + Rat::from((millis, MILLIS)))
}

fn read(s: &str, name: &str) -> Result<Rat, String> {
    READ.iter()
        .find_map(|form| read_with(s, form))
        .ok_or_else(|| {
            format!(
                "{name}: \"{s}\" is no date as \"2026-11-20\", \"2026-11-20 13:45:10\", \
             \"20-Nov-2026\" or \"11/20/2026\"; another form is given as in \
             {name}(\"20.11.2026\", \"dd.mm.yyyy\")"
            )
        })
}

/// Whether a flat list reads as [year, month, day] or [year, month, day, hour, minute, second].
fn is_vector(items: &[Expr]) -> bool {
    let parts: Vec<&Rat> = items.iter().filter_map(Expr::as_rat).collect();
    let within = |r: &Rat, lo: i64, hi: i64| *r >= lo && *r <= hi;
    matches!(items.len(), 3 | 6)
        && parts.len() == items.len()
        && parts[0].is_integer()
        && parts[1].is_integer()
        && within(parts[1], 1, 12)
        && within(parts[2], 1, 31)
        && (parts.len() == 3
            || (within(parts[3], 0, 24) && within(parts[4], 0, 60) && within(parts[5], 0, 61)))
}

/// The day number of one date: a number, a date in quotes or a list of its parts.
fn moment(e: &Expr, name: &str, parts_always: bool) -> Result<Rat, String> {
    if let Some(s) = text_of(e) {
        return read(s, name);
    }
    match e.as_list() {
        Some(items) if matches!(items.len(), 3 | 6) && (parts_always || is_vector(items)) => {
            let parts: Vec<Rat> = items
                .iter()
                .map(|x| rational(x, name))
                .collect::<Result<_, _>>()?;
            serial(&parts, name)
        }
        Some(_) => Err(format!(
            "{name} needs a day number, a date in quotes or [year, month, day] with or without \
             [hour, minute, second]"
        )),
        None => rational(e, name),
    }
}

/// A function of a date, for one date or for every date of a list.
fn each(e: &Expr, name: &str, parts_always: bool, f: &dyn Fn(&Rat) -> Res) -> Res {
    match e.as_list() {
        Some(items)
            if !(matches!(items.len(), 3 | 6)
                && items
                    .iter()
                    .all(|x| x.as_list().is_none() && text_of(x).is_none())
                && (parts_always || is_vector(items))) =>
        {
            Ok(Expr::list(
                items
                    .iter()
                    .map(|x| f(&moment(x, name, true)?))
                    .collect::<Result<_, _>>()?,
            ))
        }
        _ => f(&moment(e, name, parts_always)?),
    }
}

/// The parts [year, month, day, hour, minute, second] of a day number, the seconds exact.
fn parts(n: &Rat, name: &str) -> Res {
    let whole = floor(n);
    let day = whole
        .to_i64()
        .filter(|d| d.unsigned_abs() <= (YEARS * 366) as u64)
        .ok_or_else(|| format!("{name}: the calendar does not reach that far"))?;
    let (year, month, of_month) = civil(day);
    let seconds = Rat::from(n - &whole) * Rat::from(86_400);
    let (hours, minutes) = (
        floor(&Rat::from(&seconds / Rat::from(3600))),
        floor(&Rat::from(&seconds / Rat::from(60))),
    );
    let minute = &minutes - Integer::from(&hours * 60);
    let second = seconds - Rat::from(minutes * 60);
    let mut out: Vec<Expr> = [year, month, of_month].into_iter().map(Expr::int).collect();
    out.extend([hours, minute].into_iter().map(|x| number(Rat::from(x))));
    out.push(number(second));
    Ok(Expr::list(out))
}

/// The days of a month in rows of weeks that begin on Sunday, 0 where the month has no day.
fn month_sheet(year: i64, month: i64) -> Expr {
    let first = week_place(before(year, month) + 1) as i64;
    let last = last_day(year, month);
    Expr::list(
        (0..6)
            .map(|week| {
                Expr::list(
                    (0..7)
                        .map(|place| {
                            let day = week * 7 + place - first + 1;
                            Expr::int(if (1..=last).contains(&day) { day } else { 0 })
                        })
                        .collect(),
                )
            })
            .collect(),
    )
}

/// A day number moved by whole years or months (the day stays within its month) or by a
/// number of smaller parts.
fn moved(n: &Rat, by: &Rat, part: &str, name: &str) -> Result<Rat, String> {
    let per = match part {
        "day" => 1,
        "hour" => 24,
        "minute" => 1440,
        "second" => 86_400,
        "millisecond" => MILLIS,
        "year" | "month" => {
            let whole = floor(n);
            let (year, month, day) = civil(
                whole
                    .to_i64()
                    .filter(|d| d.unsigned_abs() <= (YEARS * 366) as u64)
                    .ok_or_else(|| format!("{name}: the calendar does not reach that far"))?,
            );
            let count = year_of(by, "number of years or months", name)?;
            let months = year * 12 + month - 1 + count * if part == "year" { 12 } else { 1 };
            let (year, month) = (months.div_euclid(12), months.rem_euclid(12) + 1);
            if year.unsigned_abs() > YEARS as u64 {
                return Err(format!("{name}: the calendar does not reach that far"));
            }
            let start = before(year, month) + day.min(last_day(year, month));
            return Ok(Rat::from(start) + Rat::from(n - &whole));
        }
        _ => {
            return Err(format!(
                "{name}(date, number, part): the part is \"year\", \"month\", \"day\", \"hour\", \
                 \"minute\", \"second\" or \"millisecond\""
            ));
        }
    };
    Ok(Rat::from(n + Rat::from(by / Rat::from(per))))
}

/// The thousandths of seconds since 1970 began, by the clock of this place.
fn local_millis() -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let seconds = now.as_secs() as libc::time_t;
    // SAFETY: a zeroed `tm` is a valid value of it (numbers and a null pointer).
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    // SAFETY: both pointers are to live values of the types that the function takes.
    let known = !unsafe { libc::localtime_r(&seconds, &mut tm) }.is_null();
    let ahead = if known { tm.tm_gmtoff as i64 } else { 0 };
    (now.as_secs() as i64 + ahead) * 1000 + i64::from(now.subsec_millis())
}

/// The thousandths of seconds for which this run has kept a processor busy.
fn busy_millis() -> i64 {
    // SAFETY: a zeroed `timespec` is a valid value of it.
    let mut spent: libc::timespec = unsafe { std::mem::zeroed() };
    // SAFETY: the pointer is to a live value of the type that the function takes.
    match unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut spent) } {
        0 => spent.tv_sec as i64 * 1000 + spent.tv_nsec as i64 / 1_000_000,
        _ => 0,
    }
}

fn today() -> Rat {
    Rat::from(EPOCH) + Rat::from((local_millis(), MILLIS))
}

/// The day number of a count of another calendar.
fn from_count(x: &Rat, kind: &str, name: &str) -> Result<Rat, String> {
    Ok(match kind {
        "datenum" => x.clone(),
        "juliandate" => Rat::from(x - Rat::from((3_442_117, 2))),
        "modifiedjuliandate" => Rat::from(x + 678_942),
        "posixtime" => Rat::from(x / Rat::from(86_400)) + Rat::from(EPOCH),
        "excel" => Rat::from(x + 693_960),
        "yyyymmdd" => {
            let count = year_of(x, "number as 20261120", name)?;
            let (year, month, day) = (count / 10_000, count / 100 % 100, count % 100);
            if !(1..=12).contains(&month) || !(1..=last_day(year, month)).contains(&day) {
                return Err(format!("{name}: {count} is no date as 20261120"));
            }
            Rat::from(before(year, month) + day)
        }
        _ => {
            return Err(format!(
                "{name}(number, kind): the kind is \"juliandate\", \"modifiedjuliandate\", \
                 \"posixtime\", \"excel\", \"yyyymmdd\" or \"datenum\""
            ));
        }
    })
}

fn inner(name: &str, a: &[Expr]) -> Result<Option<Expr>, String> {
    let n = a.len();
    let form_of = |e: &Expr| -> Result<String, String> {
        if let Some(form) = text_of(e) {
            return Ok(form.to_string());
        }
        let k = whole(e, "the number of a form of dates", 0, 31)?;
        Ok(FORMS[k as usize].to_string())
    };
    Ok(Some(match (name, n) {
        ("datenum", 1) => each(&a[0], name, true, &|d| Ok(number(d.clone())))?,
        ("datenum", 2) => {
            let kind = text_of(&a[1]).ok_or(
                "datenum(year, month, day) needs the day, datenum(date, form) the form in quotes",
            )?;
            match text_of(&a[0]) {
                Some(s) => number(read_with(s, kind).ok_or_else(|| {
                    format!("datenum: \"{s}\" is no date of the form \"{kind}\" (with a year)")
                })?),
                None => {
                    let turn = |x: &Expr| -> Res {
                        Ok(number(from_count(&rational(x, name)?, kind, name)?))
                    };
                    match a[0].as_list() {
                        Some(items) => {
                            Expr::list(items.iter().map(turn).collect::<Result<_, _>>()?)
                        }
                        None => turn(&a[0])?,
                    }
                }
            }
        }
        ("datenum", 3..=6) => {
            let parts: Vec<Rat> = a
                .iter()
                .map(|x| rational(x, name))
                .collect::<Result<_, _>>()?;
            number(serial(&parts, name)?)
        }
        ("datevec", 1) => each(&a[0], name, false, &|d| parts(d, name))?,
        ("datevec", 2) => {
            let s = text_of(&a[0]).ok_or("datevec(date, form) needs the date in quotes")?;
            let form = form_of(&a[1])?;
            let day = read_with(s, &form).ok_or_else(|| {
                format!("datevec: \"{s}\" is no date of the form \"{form}\" (with a year)")
            })?;
            parts(&day, name)?
        }
        ("datestr", 1) => each(&a[0], name, false, &|d| {
            let form = FORMS[usize::from(d.is_integer())];
            Ok(text(&written(d, form, name)?))
        })?,
        ("datestr", 2) => {
            let form = form_of(&a[1])?;
            each(&a[0], name, false, &|d| Ok(text(&written(d, &form, name)?)))?
        }
        ("weekday", 1 | 2) => {
            let kind = match a.get(1) {
                Some(k) => Some(text_of(k).ok_or("weekday(date, \"long\" | \"short\")")?),
                None => None,
            };
            each(&a[0], name, false, &|d| {
                let place = week_place(split(&Rat::from(floor(d)), name)?.0);
                Ok(match kind {
                    None => Expr::int(place as i64 + 1),
                    Some("long") => text(DAYS[place]),
                    Some("short") => text(&DAYS[place][..3]),
                    Some(_) => return Err("weekday(date, \"long\" | \"short\")".into()),
                })
            })?
        }
        ("eomday" | "leapyear" | "yeardays", _) if n == 1 + usize::from(name == "eomday") => {
            let one = |y: &Expr, m: Option<&Expr>| -> Res {
                let year = year_of(&rational(y, name)?, "year", name)?;
                Ok(match (name, m) {
                    ("eomday", Some(m)) => Expr::int(last_day(year, whole(m, "the month", 1, 12)?)),
                    ("leapyear", _) => Expr::Bool(leap(year)),
                    _ => Expr::int(365 + i64::from(leap(year))),
                })
            };
            match (a[0].as_list(), a.get(1).and_then(Expr::as_list)) {
                (Some(years), Some(months)) if years.len() == months.len() => Expr::list(
                    years
                        .iter()
                        .zip(months)
                        .map(|(y, m)| one(y, Some(m)))
                        .collect::<Result<_, _>>()?,
                ),
                (Some(_), Some(_)) => {
                    return Err("eomday needs as many months as years".into());
                }
                (Some(years), _) => Expr::list(
                    years
                        .iter()
                        .map(|y| one(y, a.get(1)))
                        .collect::<Result<_, _>>()?,
                ),
                (None, Some(months)) => Expr::list(
                    months
                        .iter()
                        .map(|m| one(&a[0], Some(m)))
                        .collect::<Result<_, _>>()?,
                ),
                (None, None) => one(&a[0], a.get(1))?,
            }
        }
        ("calendar", 0 | 1) => {
            let day = match a.first() {
                Some(d) => moment(d, name, false)?,
                None => today(),
            };
            let (year, month, _) = civil(split(&Rat::from(floor(&day)), name)?.0);
            month_sheet(year, month)
        }
        ("calendar", 2) => {
            let year = year_of(&rational(&a[0], name)?, "year", name)?;
            month_sheet(year, whole(&a[1], "the month", 1, 12)?)
        }
        ("addtodate", 3) => {
            let part =
                text_of(&a[2]).ok_or("addtodate(date, number, part) needs the part in quotes")?;
            let by = rational(&a[1], name)?;
            each(&a[0], name, false, &|d| {
                Ok(number(moved(d, &by, part, name)?))
            })?
        }
        ("etime", 2) => {
            let (later, sooner) = (moment(&a[0], name, true)?, moment(&a[1], name, true)?);
            number(Rat::from(later - sooner) * Rat::from(86_400))
        }
        ("juliandate", 1) => each(&a[0], name, false, &|d| {
            Ok(number(Rat::from(d + Rat::from((3_442_117, 2)))))
        })?,
        ("mjuliandate", 1) => each(&a[0], name, false, &|d| Ok(number(Rat::from(d - 678_942))))?,
        ("posixtime", 1) => each(&a[0], name, false, &|d| {
            Ok(number(Rat::from(d - EPOCH) * Rat::from(86_400)))
        })?,
        ("dayofyear", 1) => each(&a[0], name, false, &|d| {
            let day = split(&Rat::from(floor(d)), name)?.0;
            Ok(Expr::int(day - before(civil(day).0, 1)))
        })?,
        ("isweekend", 1) => each(&a[0], name, false, &|d| {
            let place = week_place(split(&Rat::from(floor(d)), name)?.0);
            Ok(Expr::Bool(place == 0 || place == 6))
        })?,
        ("now", 0) => number(today()),
        ("clock", 0) => parts(&today(), name)?,
        ("date", 0) => text(&written(&today(), FORMS[1], name)?),
        ("cputime", 0) => number(Rat::from((busy_millis(), 1000))),
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
    fn days_and_dates_there_and_back() {
        assert_eq!(before(0, 1) + 1, 1);
        assert_eq!(before(1970, 1) + 1, EPOCH);
        assert_eq!(before(2000, 3) + 1, 730_546);
        assert_eq!(before(2001, 14), before(2002, 2));
        assert_eq!(before(2001, -5), before(2001, 1));
        let mut last = civil(-800_000);
        for day in -799_999..800_000 {
            let (year, month, of_month) = civil(day);
            assert_eq!(before(year, month) + of_month, day);
            let next = match last {
                (y, 12, 31) => (y + 1, 1, 1),
                (y, m, d) if d == last_day(y, m) => (y, m + 1, 1),
                (y, m, d) => (y, m, d + 1),
            };
            assert_eq!((year, month, of_month), next);
            last = next;
        }
        assert_eq!(civil(EPOCH), (1970, 1, 1));
        assert_eq!(week_place(EPOCH), 4);
    }

    #[test]
    fn forms_are_written_and_read() {
        let day = Rat::from((740_306i64 * 86_400 + 13 * 3600 + 45 * 60 + 10, 86_400));
        for (k, wanted) in [
            (0, "20-Nov-2026 13:45:10"),
            (1, "20-Nov-2026"),
            (2, "11/20/26"),
            (12, "Nov26"),
            (14, " 1:45:10 PM"),
            (16, " 1:45 PM"),
            (17, "Q4-26"),
            (21, "Nov.20,2026 13:45:10"),
            (27, "Q4-2026"),
            (30, "20261120T134510"),
            (31, "2026-11-20 13:45:10"),
        ] {
            assert_eq!(written(&day, FORMS[k], "datestr").as_deref(), Ok(wanted));
        }
        assert_eq!(
            written(&day, "dddd, d mmmm m yyyy FFF", "datestr").as_deref(),
            Ok("Friday, F November N 2026 000")
        );
        for form in [0, 21, 30, 31] {
            let text = written(&day, FORMS[form], "datestr").expect("a date");
            assert_eq!(read_with(&text, FORMS[form]), Some(day.clone()), "{text}");
            assert_eq!(read(&text, "datenum"), Ok(day.clone()), "{text}");
        }
        assert_eq!(read_with("5.2.99", "dd.mm.yy"), Some(Rat::from(730_156)));
        assert_eq!(
            read_with("12:30 AM 1/2/2000", "HH:MM PM mm/dd/yyyy").map(|d| floor(&d)),
            Some(Integer::from(730_487))
        );
        assert!(read("2026-02-30", "datenum").is_err());
        assert!(read("2026-11-20 24:00:00", "datenum").is_err());
        assert!(read_with("11/20", "mm/dd").is_none());
        // a half of a thousandth of a second before midnight is the next day
        let late = Rat::from(740_307) - Rat::from((1, 2 * MILLIS));
        assert_eq!(
            written(&late, FORMS[31], "datestr").as_deref(),
            Ok("2026-11-21 00:00:00")
        );
    }

    #[test]
    fn the_clock_of_this_place_runs() {
        let now = today();
        assert!(now > 739_000 && now < 800_000);
        assert!(busy_millis() >= 0);
    }
}
