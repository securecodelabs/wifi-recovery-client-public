use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Parser)]
#[command(version, about)]
pub struct Options {
    #[arg(
        long,
        env = "WIFI_RECOVERY_URL",
        default_value = "http://127.0.0.1:8080",
        global = true
    )]
    pub server: String,
    #[arg(long, global = true)]
    pub json: bool,
    #[arg(long, default_value="2", global=true, value_parser=interval)]
    pub interval: f64,
    #[command(subcommand)]
    pub command: Action,
}

#[derive(Subcommand)]
pub enum Action {
    Info {
        #[arg(long)]
        watch: bool,
    },
    Pattern {
        #[command(subcommand)]
        command: Pattern,
    },
    Submit(Submission),
    Jobs,
    Status {
        id: String,
    },
    Watch {
        id: String,
    },
    Stop {
        id: String,
    },
    Resume {
        id: String,
    },
    Result {
        id: String,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    Delete {
        id: String,
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
pub enum Pattern {
    Preview { pattern: String },
}

#[derive(Args)]
pub struct Submission {
    pub file: PathBuf,
    #[arg(long)]
    pub pattern: String,
    #[arg(long, default_value="0", value_parser=duration)]
    pub runtime: u64,
    #[arg(long, default_value_t = 0)]
    pub limit: u64,
    #[arg(long)]
    pub request_id: Option<Uuid>,
}

fn interval(text: &str) -> Result<f64, String> {
    text.parse::<f64>()
        .ok()
        .filter(|n| n.is_finite() && (0.1..=3600.0).contains(n))
        .ok_or_else(|| "Polling interval must be 0.1-3600 seconds".into())
}

fn duration(text: &str) -> Result<u64, String> {
    let (number, multiplier) = match text.as_bytes().last() {
        Some(b's') => (&text[..text.len() - 1], 1),
        Some(b'm') => (&text[..text.len() - 1], 60),
        Some(b'h') => (&text[..text.len() - 1], 3600),
        _ => (text, 1),
    };
    number
        .parse::<u64>()
        .ok()
        .and_then(|value| value.checked_mul(multiplier))
        .filter(|value| *value <= 86400)
        .ok_or_else(|| {
            "Runtime must be 0-86400 whole seconds, optionally using s, m or h (0 is unlimited)"
                .into()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_durations_and_intervals() {
        assert_eq!(duration("2h").unwrap(), 7200);
        assert_eq!(duration("0").unwrap(), 0);
        assert_eq!(duration("24h").unwrap(), 86400);
        for invalid in ["1.5h", "-2", "18446744073709551615h", "86401", "25h"] {
            assert!(duration(invalid).is_err());
        }
        for invalid in ["NaN", "inf", "0", "3601"] {
            assert!(interval(invalid).is_err());
        }
    }
}
