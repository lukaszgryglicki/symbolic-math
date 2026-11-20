use std::io::{BufRead, IsTerminal, Write};
use std::process::ExitCode;
use symbolic_math::assume::Domain;
use symbolic_math::{Session, names};

/// The text of `--help`: the options, and an example of every kind of operation.
const USAGE: &str = include_str!("help.txt");

/// A long text for the standard output; a reader that stops early (`| head`) is no failure.
fn shown(text: &str) -> ExitCode {
    let _ = std::io::stdout().lock().write_all(text.as_bytes());
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let mut session = Session::new();
    let mut inputs = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
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
            "-d" | "--digits" => match args
                .next()
                .and_then(|v| v.parse::<u32>().ok())
                .filter(|d| (1..=1_000_000).contains(d))
            {
                Some(d) => session.digits = Some(d),
                None => {
                    eprintln!("error: --digits needs a whole number from 1 to 1000000");
                    return ExitCode::from(2);
                }
            },
            _ if arg.starts_with("--digits=") => match arg["--digits=".len()..]
                .parse::<u32>()
                .ok()
                .filter(|d| (1..=1_000_000).contains(d))
            {
                Some(d) => session.digits = Some(d),
                None => {
                    eprintln!("error: --digits needs a whole number from 1 to 1000000");
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
