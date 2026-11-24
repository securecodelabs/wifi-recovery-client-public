use clap::Parser;
use std::{io::Write, process::ExitCode};
use wifi_recovery_client::{args::Options, commands, output};

#[tokio::main]
async fn main() -> ExitCode {
    if std::env::args_os()
        .nth(1)
        .is_some_and(|arg| arg == "--licenses")
    {
        return if write!(
            std::io::stdout().lock(),
            "{}\n{}",
            include_str!("../LICENSE"),
            include_str!("../THIRD_PARTY_NOTICES.md")
        )
        .is_ok()
        {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }
    match commands::run(Options::parse()).await {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            let _ = writeln!(
                std::io::stderr().lock(),
                "{}",
                output::escaped(&format!("{error:#}"))
            );
            ExitCode::FAILURE
        }
    }
}
