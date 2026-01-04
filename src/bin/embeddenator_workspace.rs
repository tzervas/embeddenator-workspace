use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("docs") => docs(),
        Some("rustdoc") => rustdoc(),
        Some("mdbook") => mdbook(),
        Some("help") | None => {
            eprintln!("embeddenator-workspace\n\nCommands:\n  docs     Run rustdoc + mdBook (if installed)\n  rustdoc  Run ./generate_docs.sh\n  mdbook   Run ./scripts/docs/build_mdbook.sh\n");
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("Unknown command: {other}");
            ExitCode::from(2)
        }
    }
}

fn run(cmd: &mut Command) -> ExitCode {
    match cmd.status() {
        Ok(st) if st.success() => ExitCode::SUCCESS,
        Ok(st) => ExitCode::from(st.code().unwrap_or(1) as u8),
        Err(e) => {
            eprintln!("Failed to run command: {e}");
            ExitCode::from(1)
        }
    }
}

fn rustdoc() -> ExitCode {
    let mut cmd = Command::new("bash");
    cmd.arg("./generate_docs.sh");
    run(&mut cmd)
}

fn mdbook() -> ExitCode {
    let mut cmd = Command::new("bash");
    cmd.arg("./scripts/docs/build_mdbook.sh");
    run(&mut cmd)
}

fn docs() -> ExitCode {
    let rc = rustdoc();
    if rc != ExitCode::SUCCESS {
        return rc;
    }

    // mdBook is optional; if not installed, the script exits nonzero. Treat that as non-fatal.
    let mut cmd = Command::new("bash");
    cmd.arg("./scripts/docs/build_mdbook.sh");
    match cmd.status() {
        Ok(st) if st.success() => ExitCode::SUCCESS,
        Ok(_) => {
            eprintln!("Note: mdBook not built (mdbook not installed?)");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Note: mdBook not built: {e}");
            ExitCode::SUCCESS
        }
    }
}
