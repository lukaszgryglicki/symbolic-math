//! A time limit for searches that may run without end.

use std::cell::Cell;
use std::time::{Duration, Instant};

thread_local! {
    static DEADLINE: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// Runs `f`; inside it `expired()` turns true after `limit` (or at an outer limit that ends sooner).
pub fn with_limit<T>(limit: Duration, f: impl FnOnce() -> T) -> T {
    let outer = DEADLINE.get();
    let mine = Instant::now() + limit;
    DEADLINE.set(Some(outer.map_or(mine, |o| o.min(mine))));
    let out = f();
    DEADLINE.set(outer);
    out
}

pub fn expired() -> bool {
    DEADLINE.get().is_some_and(|d| Instant::now() >= d)
}
