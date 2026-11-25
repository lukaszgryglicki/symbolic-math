use std::io::{BufRead, IsTerminal, Write};
use std::process::ExitCode;
use symbolic_math::assume::Domain;
use symbolic_math::{Session, names, settings};

/// The text of `--help`: the options, and an example of every kind of operation.
const USAGE: &str = include_str!("help.txt");

/// A long text for the standard output; a reader that stops early (`| head`) is no failure.
fn shown(text: &str) -> ExitCode {
    let _ = std::io::stdout().lock().write_all(text.as_bytes());
    ExitCode::SUCCESS
}

/// The memory that the program takes when nothing is said: as many megabytes at most, and
/// no more than three quarters of the memory of the machine. A calculation without end (a
/// loop that squares a number again and again, a power of a sum with millions of terms)
/// then ends with an error, and the machine keeps its memory for its other programs.
const USUAL_MEMORY: u64 = 8192;

/// The megabytes that the program takes at most when nothing is said, on a machine with
/// `physical` megabytes of memory (`None`: not known).
fn usual_megabytes(physical: Option<u64>) -> u64 {
    match physical {
        Some(all) => USUAL_MEMORY.min(all / 4 * 3).max(256),
        None => USUAL_MEMORY,
    }
}

/// The megabytes of memory of the machine.
fn physical_megabytes() -> Option<u64> {
    // SAFETY: `sysconf` takes a number and gives one.
    let (pages, size) = unsafe {
        (
            libc::sysconf(libc::_SC_PHYS_PAGES),
            libc::sysconf(libc::_SC_PAGESIZE),
        )
    };
    let bytes = u64::try_from(pages)
        .ok()?
        .checked_mul(u64::try_from(size).ok()?)?;
    Some(bytes >> 20).filter(|megabytes| *megabytes > 0)
}

/// What the program says when the memory that it may take has ended, as bytes that are ready
/// before it happens: then nothing can be put together any more.
static OUT_OF_MEMORY: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();

/// The end of a run that has no memory left: the message, and the exit status of an error.
fn out_of_memory() -> ! {
    const PLAIN: &[u8] = b"error: out of memory\n";
    let text = OUT_OF_MEMORY.get().map_or(PLAIN, Vec::as_slice);
    // SAFETY: `write` reads the bytes of a slice that lives; `_exit` ends the process without
    // running anything that would need memory.
    unsafe {
        libc::write(2, text.as_ptr().cast(), text.len());
        libc::_exit(1)
    }
}

/// The allocator of the system, with an end of its own where it gives no memory: an error
/// like any other, not the abort of the run with a core.
struct Bounded;

// SAFETY: every call goes to the allocator of the system as it is; a null pointer never
// comes back from here, because the run ends in its place.
unsafe impl std::alloc::GlobalAlloc for Bounded {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        let block = unsafe { std::alloc::System.alloc(layout) };
        if block.is_null() {
            out_of_memory();
        }
        block
    }

    unsafe fn alloc_zeroed(&self, layout: std::alloc::Layout) -> *mut u8 {
        let block = unsafe { std::alloc::System.alloc_zeroed(layout) };
        if block.is_null() {
            out_of_memory();
        }
        block
    }

    unsafe fn realloc(&self, old: *mut u8, layout: std::alloc::Layout, size: usize) -> *mut u8 {
        let block = unsafe { std::alloc::System.realloc(old, layout, size) };
        if block.is_null() {
            out_of_memory();
        }
        block
    }

    unsafe fn dealloc(&self, block: *mut u8, layout: std::alloc::Layout) {
        unsafe { std::alloc::System.dealloc(block, layout) }
    }
}

#[global_allocator]
static MEMORY: Bounded = Bounded;

type Bytes = *mut std::ffi::c_void;

unsafe extern "C" {
    /// `mp_set_memory_functions` of GMP, by the name that its library has for it.
    fn __gmp_set_memory_functions(
        alloc: Option<unsafe extern "C" fn(usize) -> Bytes>,
        realloc: Option<unsafe extern "C" fn(Bytes, usize, usize) -> Bytes>,
        free: Option<unsafe extern "C" fn(Bytes, usize)>,
    );
}

/// Memory for the numbers of GMP, MPFR and MPC: where there is none, GMP would print a
/// message of its own and abort.
unsafe extern "C" fn number_alloc(size: usize) -> Bytes {
    // SAFETY: `malloc` of a size; the pointer is checked.
    let block = unsafe { libc::malloc(size) };
    if block.is_null() {
        out_of_memory();
    }
    block
}

unsafe extern "C" fn number_realloc(old: Bytes, _before: usize, size: usize) -> Bytes {
    // SAFETY: `old` is a block of `number_alloc` or of this function, as GMP hands it back.
    let block = unsafe { libc::realloc(old, size) };
    if block.is_null() {
        out_of_memory();
    }
    block
}

unsafe extern "C" fn number_free(block: Bytes, _size: usize) {
    // SAFETY: `block` is a block of `number_alloc` or `number_realloc`.
    unsafe { libc::free(block) }
}

/// The most megabytes that the program may take, and who said so.
///
/// `SYMBOLIC_MATH_MEMORY=N`: the address space of the program is N megabytes at most, so that
/// a calculation that needs more ends with an error and does not take the memory of the
/// machine; `SYMBOLIC_MATH_MEMORY=unlimited` (or `none`): as much as the system gives.
/// Without the variable: `usual_megabytes`, where the system has no smaller limit already.
fn limit_memory() -> Result<(), String> {
    // SAFETY: the three functions are the ones that GMP is to call from now on; nothing of
    // GMP is in use before this, the first thing that the program does.
    unsafe {
        __gmp_set_memory_functions(Some(number_alloc), Some(number_realloc), Some(number_free));
    }
    // (the type of a limit has a sign on FreeBSD and none on Linux and macOS: through a
    // type that holds both)
    let megabytes = |limit: libc::rlim_t| {
        (limit != libc::RLIM_INFINITY).then_some(((limit as i128).max(0) >> 20) as u64)
    };
    let mut now = libc::rlimit {
        rlim_cur: libc::RLIM_INFINITY,
        rlim_max: libc::RLIM_INFINITY,
    };
    // SAFETY: `getrlimit` writes the struct, which lives until the call returns.
    let known = unsafe { libc::getrlimit(libc::RLIMIT_AS, &mut now) } == 0;
    let set = |limit: &libc::rlimit| -> bool {
        // SAFETY: `setrlimit` reads the struct, which lives until the call returns.
        unsafe { libc::setrlimit(libc::RLIMIT_AS, limit) == 0 }
    };
    let asked = std::env::var("SYMBOLIC_MATH_MEMORY").ok();
    let limit = match asked.as_deref().map(str::trim) {
        Some("unlimited" | "none") => megabytes(now.rlim_cur).filter(|_| known),
        Some(value) => {
            let megabytes = value
                .parse::<u64>()
                .ok()
                .filter(|m| (64..=1 << 30).contains(m))
                .ok_or(
                    "SYMBOLIC_MATH_MEMORY needs megabytes: a whole number from 64 on, or unlimited",
                )?;
            let limit = libc::rlimit {
                rlim_cur: (megabytes << 20) as libc::rlim_t,
                rlim_max: (megabytes << 20) as libc::rlim_t,
            };
            if !set(&limit) {
                return Err(format!(
                    "SYMBOLIC_MATH_MEMORY: this system does not take the limit ({})",
                    std::io::Error::last_os_error()
                ));
            }
            Some(megabytes)
        }
        None => {
            // (only ever less than there was: a limit of the shell, or of the service that
            // runs the program, stays; a system that takes none goes without)
            let usual = usual_megabytes(physical_megabytes());
            match megabytes(now.rlim_cur).filter(|_| known) {
                Some(already) if already <= usual => Some(already),
                _ => {
                    let limit = libc::rlimit {
                        rlim_cur: ((usual << 20) as libc::rlim_t).min(now.rlim_max),
                        rlim_max: now.rlim_max,
                    };
                    (known && set(&limit))
                        .then(|| megabytes(limit.rlim_cur))
                        .flatten()
                }
            }
        }
    };
    let _ = OUT_OF_MEMORY.set(
        match limit {
            Some(megabytes) => format!(
                "error: out of memory: the calculation needs more than the {megabytes} megabytes that the program may take (SYMBOLIC_MATH_MEMORY=N sets another limit, SYMBOLIC_MATH_MEMORY=unlimited none)\n"
            ),
            None => "error: out of memory\n".to_string(),
        }
        .into_bytes(),
    );
    Ok(())
}

/// The stack of the thread that works: room for a function that calls itself some thousand
/// times, one call inside the other.
const STACK: usize = 64 << 20;

fn main() -> ExitCode {
    if let Err(why) = limit_memory() {
        eprintln!("error: {why}");
        return ExitCode::from(2);
    }
    let worker = std::thread::Builder::new().stack_size(STACK).spawn(|| {
        // (a quarter of the stack is left to the work inside the innermost call)
        symbolic_math::eval::stack_for_calls(STACK - STACK / 4);
        work()
    });
    match worker {
        Ok(thread) => thread.join().unwrap_or(ExitCode::from(101)),
        // (no memory for such a stack: the stack of the program itself, and fewer calls)
        Err(_) => work(),
    }
}

/// Reads the options and runs the inputs.
fn work() -> ExitCode {
    let mut session = Session::new();
    let mut inputs = Vec::new();
    let mut args = std::env::args().skip(1);
    // the steps of verbose are printed as they are told
    symbolic_math::trace::set_sink(Some(Box::new(|line| {
        let mut out = std::io::stdout().lock();
        if writeln!(out, "{line}").is_err() || out.flush().is_err() {
            std::process::exit(0);
        }
    })));
    let whole = |text: Option<String>, most: u64| -> Option<u64> {
        text.and_then(|v| v.parse::<u64>().ok())
            .filter(|n| (1..=most).contains(n))
    };
    while let Some(arg) = args.next() {
        // --name=value is --name value
        let (name, mut value) = match arg.split_once('=') {
            Some((name, value)) if name.starts_with("--") => (name, Some(value.to_string())),
            _ => (arg.as_str(), None),
        };
        // (the steps: on with the bare option, or as the value says; the other options that
        // switch something on take no value)
        let level = match (name, value.as_deref()) {
            ("-v" | "--verbose", None) => Some(1),
            ("--debug", None) => Some(2),
            ("--verbose" | "--debug", Some("off" | "no" | "false" | "none" | "default" | "0")) => {
                Some(0)
            }
            ("--verbose", Some("on" | "yes" | "true" | "1")) => Some(1),
            ("--debug", Some("on" | "yes" | "true")) => Some(2),
            ("--verbose" | "--debug", Some("all" | "2")) => Some(2),
            ("--debug", Some("1")) => Some(1),
            ("--verbose" | "--debug", Some(other)) => {
                eprintln!(
                    "error: {name}={other}: the value is 0, 1 or 2 (off, on, all): no steps, the steps, every detail"
                );
                return ExitCode::from(2);
            }
            _ => None,
        };
        if let Some(level) = level {
            symbolic_math::trace::set_level(level);
            continue;
        }
        if matches!(
            name,
            "--help"
                | "--version"
                | "--functions"
                | "--constants"
                | "--units"
                | "--real"
                | "--quaternions"
                | "--numeric"
                | "--exact"
        ) && value.is_some()
        {
            eprintln!("error: {name} takes no value");
            return ExitCode::from(2);
        }
        let mut given =
            |args: &mut dyn Iterator<Item = String>| value.take().or_else(|| args.next());
        match name {
            "-h" | "--help" => return shown(USAGE),
            "-V" | "--version" => {
                println!("symbolic-math {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            "--functions" => return shown(&names::functions_help()),
            "--constants" => return shown(&names::constants_help()),
            "--units" => return shown(&symbolic_math::units::help()),
            "-r" | "--real" => session.set_domain(Domain::Real),
            "-q" | "--quaternions" => session.set_domain(Domain::Quaternion),
            "-n" | "--numeric" => session.numeric = true,
            "-e" | "--exact" => session.exact = true,
            "-d" | "--digits" | "--prec" | "--precision" => {
                match whole(given(&mut args), u64::from(settings::MAX_DIGITS)) {
                    Some(d) => session.digits = Some(d as u32),
                    None => {
                        eprintln!("error: --digits needs a whole number from 1 to 1000000");
                        return ExitCode::from(2);
                    }
                }
            }
            "--roots" => match whole(given(&mut args), u64::from(settings::MAX_ROOTS)) {
                Some(n) => settings::set_roots(Some(n as u32)),
                None => {
                    eprintln!(
                        "error: --roots needs a whole number from 1 to {}",
                        settings::MAX_ROOTS
                    );
                    return ExitCode::from(2);
                }
            },
            "--terms" => match whole(given(&mut args), settings::MAX_TERMS) {
                Some(n) => settings::set_terms(Some(n)),
                None => {
                    eprintln!(
                        "error: --terms needs a whole number from 1 to {}",
                        settings::MAX_TERMS
                    );
                    return ExitCode::from(2);
                }
            },
            "--" => inputs.extend(args.by_ref()),
            _ => inputs.push(arg),
        }
    }
    let mut failed = false;
    let mut run = |session: &mut Session, input: &str| match session.run(input) {
        Ok(lines) => {
            let mut out = std::io::stdout().lock();
            for line in lines {
                if writeln!(out, "{line}").is_err() {
                    std::process::exit(0);
                }
            }
        }
        Err(err) => {
            eprintln!("error: {err}");
            failed = true;
        }
    };
    if inputs.is_empty() {
        let stdin = std::io::stdin();
        let interactive = stdin.is_terminal();
        loop {
            if interactive {
                print!("> ");
                let _ = std::io::stdout().flush();
            }
            let mut line = String::new();
            match stdin.lock().read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
            let line = line.trim();
            if matches!(line, "quit" | "exit") {
                break;
            }
            if !line.is_empty() && !line.starts_with('#') {
                run(&mut session, line);
            }
        }
    } else {
        for input in &inputs {
            run(&mut session, input);
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_memory_that_is_usual_follows_the_machine() {
        // (megabytes: three quarters of a small machine, 8192 on a large one, 256 at least)
        assert_eq!(usual_megabytes(None), 8192);
        assert_eq!(usual_megabytes(Some(65536)), 8192);
        assert_eq!(usual_megabytes(Some(16384)), 8192);
        assert_eq!(usual_megabytes(Some(8192)), 6144);
        assert_eq!(usual_megabytes(Some(4096)), 3072);
        assert_eq!(usual_megabytes(Some(1024)), 768);
        assert_eq!(usual_megabytes(Some(128)), 256);
        assert!(physical_megabytes().is_none_or(|megabytes| megabytes >= 64));
    }
}
