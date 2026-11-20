//! Files in a sandbox. Where the program runs for strangers, as behind a web page, the
//! environment variable `SYMBOLIC_MATH_SANDBOX` keeps what it reads and writes inside the
//! current directory, under plain names, and bounds how much it writes: the megabytes that
//! the variable names, 32 for any other text. Without the variable nothing is restricted.

use std::sync::atomic::{AtomicU64, Ordering};

/// The files that one run may write in a sandbox.
const FILES: u64 = 64;

static WRITTEN: AtomicU64 = AtomicU64::new(0);
static COUNT: AtomicU64 = AtomicU64::new(0);

/// The bytes that may be written, when the sandbox is on.
fn limit() -> Option<u64> {
    let value = std::env::var("SYMBOLIC_MATH_SANDBOX").ok()?;
    megabytes(&value).map(|m| m << 20)
}

/// Whether the program runs in a sandbox.
pub fn active() -> bool {
    limit().is_some()
}

fn megabytes(value: &str) -> Option<u64> {
    match value.trim() {
        "" => None,
        text => Some(text.parse().ok().filter(|m| (1..=4096).contains(m)).unwrap_or(32)),
    }
}

/// A name of a file of the current directory and nothing else: letters, digits, `-`, `_`
/// and `.`, not beginning with a dot.
fn plain(name: &str) -> bool {
    (1..=96).contains(&name.len())
        && !name.starts_with('.')
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
}

/// Whether the file may be read or written: always without a sandbox, in one only a plain
/// name of the current directory.
pub fn allowed(path: &str) -> Result<(), String> {
    match limit().is_some() && !plain(path) {
        true => Err(format!(
            "{path:?} is no file that may be used here: this program runs in a sandbox, where a file has a plain name as \"result.png\" (letters, digits, - _ and ., no directories)"
        )),
        false => Ok(()),
    }
}

/// Counts a file of that many bytes that is about to be written; refuses it beyond what a
/// sandbox allows.
pub fn spend(bytes: usize) -> Result<(), String> {
    let Some(limit) = limit() else {
        return Ok(());
    };
    let total = WRITTEN.fetch_add(bytes as u64, Ordering::SeqCst) + bytes as u64;
    let count = COUNT.fetch_add(1, Ordering::SeqCst) + 1;
    if total > limit {
        return Err(format!(
            "the files of this run would take more than the {} MiB that the sandbox allows",
            limit >> 20
        ));
    }
    if count > FILES {
        return Err(format!("the sandbox allows {FILES} files for one run"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_plain_names_are_files_of_a_sandbox() {
        for name in ["result.png", "a", "walk-3d_2.jxl", "data.csv", "X1.TXT"] {
            assert!(plain(name), "{name}");
        }
        for name in [
            "", ".", "..", ".hidden", "../x.png", "/tmp/x.png", "dir/x.png", "a b.png", "x\\y", "~x",
            "x;y", "x\ny", "ż.png", "a/../b",
        ] {
            assert!(!plain(name), "{name}");
        }
        assert!(!plain(&"x".repeat(97)));
    }

    #[test]
    fn the_variable_names_megabytes() {
        assert_eq!(megabytes(""), None);
        assert_eq!(megabytes("  "), None);
        assert_eq!(megabytes("16"), Some(16));
        assert_eq!(megabytes("1"), Some(1));
        assert_eq!(megabytes("yes"), Some(32));
        assert_eq!(megabytes("0"), Some(32));
        assert_eq!(megabytes("99999"), Some(32));
    }
}
