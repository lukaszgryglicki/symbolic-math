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

/// `SYMBOLIC_MATH_MEMORY=N`: the address space of the program is N megabytes at most, so that
/// a calculation that needs more ends with an error and does not take the memory of the machine.
fn limit_memory() -> Result<(), String> {
    let Ok(value) = std::env::var("SYMBOLIC_MATH_MEMORY") else {
        return Ok(());
    };
    let megabytes = value
        .trim()
        .parse::<u64>()
        .ok()
        .filter(|m| (64..=1 << 30).contains(m))
        .ok_or("SYMBOLIC_MATH_MEMORY needs megabytes: a whole number from 64 on")?;
    let limit = libc::rlimit {
        rlim_cur: (megabytes << 20) as libc::rlim_t,
        rlim_max: (megabytes << 20) as libc::rlim_t,
    };
    // SAFETY: `setrlimit` reads the struct, which lives until the call returns.
    match unsafe { libc::setrlimit(libc::RLIMIT_AS, &limit) } {
        0 => Ok(()),
        _ => Err(format!(
            "SYMBOLIC_MATH_MEMORY: this system does not take the limit ({})",
            std::io::Error::last_os_error()
        )),
    }
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
            "-v" | "--verbose" => symbolic_math::trace::set_level(1),
            "--debug" => symbolic_math::trace::set_level(2),
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
