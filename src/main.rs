use clap::Parser;
use std::process::ExitCode;
use wifi_recovery_client::{args::Options, commands, output};

#[tokio::main]
async fn main() -> ExitCode {
    if std::env::args_os()
        .nth(1)
        .is_some_and(|arg| arg == "--licenses")
    {
        print!(
            "{}\n{}",
            include_str!("../LICENSE"),
            include_str!("../THIRD_PARTY_NOTICES.md")
        );
        return ExitCode::SUCCESS;
    }
    match commands::run(Options::parse()).await {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("{}", output::escaped(&format!("{error:#}")));
            ExitCode::FAILURE
        }
    }
}
