use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::Value;
use std::{io::Write, path::Path};
use uuid::Uuid;

pub fn escaped(text: &str) -> String {
    text.chars()
        .flat_map(|c| {
            if c.is_control() {
                c.escape_default().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}

fn field(value: &Value, name: &str) -> String {
    match &value[name] {
        Value::Null => "unknown".into(),
        Value::String(text) => escaped(text),
        value => value.to_string(),
    }
}

fn timestamp(value: &Value, name: &str) -> String {
    value[name]
        .as_u64()
        .and_then(|ms| {
            time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000).ok()
        })
        .and_then(|date| {
            date.format(&time::format_description::well_known::Rfc3339)
                .ok()
        })
        .unwrap_or_else(|| "unknown".into())
}

pub fn print(value: &Value, json: bool) -> Result<()> {
    let mut output = std::io::stdout().lock();
    if json {
        writeln!(output, "{}", serde_json::to_string(value)?)?;
    } else if value.get("job").is_some() {
        job(&value["job"], &mut output)?;
        resources(&value["system"], &mut output)?;
    } else if value.get("search").is_some() {
        job(value, &mut output)?;
    } else if value.get("resources").is_some() {
        resources(value, &mut output)?;
    } else {
        writeln!(output, "{}", serde_json::to_string_pretty(value)?)?;
    }
    Ok(())
}

fn incremental_lengths(search: &Value, output: &mut impl Write) -> Result<()> {
    if search["increment"].as_bool().unwrap_or(false) {
        writeln!(
            output,
            "Incremental password lengths (bytes): {}..{}",
            field(search, "increment_min"),
            field(search, "increment_max")
        )?;
    }
    Ok(())
}

fn job(value: &Value, output: &mut impl Write) -> Result<()> {
    writeln!(output, "{}  {}", field(value, "id"), field(value, "state"))?;
    writeln!(
        output,
        "Pattern: {}  File: {}",
        field(&value["search"], "pattern"),
        field(value, "filename")
    )?;
    incremental_lengths(&value["search"], output)?;
    writeln!(
        output,
        "Created / first started (UTC): {} / {}",
        timestamp(value, "created_at_ms"),
        timestamp(value, "first_started_at_ms")
    )?;
    let progress = &value["progress"];
    writeln!(
        output,
        "Stage: {}  Fresh: {}  Sample (UTC): {}",
        field(progress, "stage"),
        field(progress, "fresh"),
        timestamp(progress, "sample_at_ms")
    )?;
    writeln!(
        output,
        "Candidates: {}/{}  Fraction: {}  Recovered: {}/{}",
        field(progress, "tested_candidates"),
        field(progress, "total_candidates"),
        field(progress, "fraction"),
        field(progress, "recovered_records"),
        field(progress, "total_records")
    )?;
    writeln!(
        output,
        "Latest batch candidate: {}  Hex: {}",
        field(progress, "candidate_text"),
        field(progress, "candidate_hex")
    )?;
    writeln!(
        output,
        "kH/s (engine run average): {}  Candidates/s: {}",
        field(progress, "kh_per_second"),
        field(progress, "candidates_per_second")
    )?;
    writeln!(
        output,
        "Active / search elapsed (s): {} / {}",
        field(value, "total_active_seconds"),
        field(progress, "search_elapsed_seconds")
    )?;
    writeln!(
        output,
        "Estimated full search / remaining (s): {} / {}  Finish (UTC): {}",
        field(progress, "estimated_full_seconds"),
        field(progress, "search_eta_seconds"),
        timestamp(progress, "estimated_finish_at_ms")
    )?;
    writeln!(
        output,
        "Runtime / candidate budget remaining: {} / {}",
        field(progress, "runtime_remaining_seconds"),
        field(progress, "candidate_limit_remaining")
    )?;
    if let Some(attempt) = value["attempts"].as_array().and_then(|items| items.last()) {
        writeln!(
            output,
            "Attempt {}: started {}  search started {}  elapsed {} s  timing complete {}",
            field(attempt, "number"),
            timestamp(attempt, "started_at_ms"),
            timestamp(attempt, "search_started_at_ms"),
            field(attempt, "elapsed_seconds"),
            field(attempt, "timing_complete")
        )?;
    }
    writeln!(
        output,
        "Resume available: {}  {}",
        field(value, "resume_available"),
        field(value, "resume_reason")
    )?;
    if let Some(error) = value["error"].as_str() {
        writeln!(output, "Error: {}", escaped(error))?;
    }
    Ok(())
}

fn resources(value: &Value, output: &mut impl Write) -> Result<()> {
    let r = &value["resources"];
    writeln!(
        output,
        "CPU: {}%  RAM used/available/total (bytes): {}/{}/{}  Sample age: {} ms",
        field(r, "cpu_percent"),
        field(r, "memory_used_bytes"),
        field(r, "memory_available_bytes"),
        field(r, "memory_total_bytes"),
        field(value, "sample_age_ms")
    )?;
    writeln!(
        output,
        "Selected GPU: {} ({})",
        field(r, "selected_gpu_pci"),
        field(r, "gpu_mapping")
    )?;
    if let Some(gpus) = r["gpus"].as_array() {
        for gpu in gpus {
            writeln!(
                output,
                "GPU {}: load {}%  VRAM used/total (bytes): {}/{}",
                field(gpu, "pci"),
                field(gpu, "utilization_percent"),
                field(gpu, "memory_used_bytes"),
                field(gpu, "memory_total_bytes")
            )?;
        }
    }
    Ok(())
}

#[derive(Deserialize)]
struct ResultEnvelope {
    contract: u32,
    job_id: Uuid,
    engine_result: EngineResult,
}
#[derive(Deserialize)]
struct EngineResult {
    schema: u32,
    recovered: usize,
    total_records: usize,
    matches: Vec<VerifiedMatch>,
}
#[derive(Deserialize)]
struct VerifiedMatch {
    verified: bool,
    password_hex: String,
}

pub fn validate_result(value: &Value, id: Uuid) -> Result<()> {
    let result: ResultEnvelope = serde_json::from_value(value.clone())?;
    ensure!(
        result.contract == 1 && result.job_id == id,
        "Result belongs to a different job or contract"
    );
    let engine = result.engine_result;
    ensure!(
        engine.schema == 1
            && (1..=10000).contains(&engine.total_records)
            && engine.recovered == engine.matches.len()
            && engine.recovered <= engine.total_records,
        "Invalid verified result counts"
    );
    for found in engine.matches {
        ensure!(
            found.verified && (8..=63).contains(&hex::decode(found.password_hex)?.len()),
            "Result includes an unverified password"
        );
    }
    Ok(())
}

pub fn export(value: &Value, path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    ensure!(parent.is_dir(), "Output directory does not exist");
    let bytes = serde_json::to_vec_pretty(value)?;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .context("Cannot create output; existing files are never overwritten")?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}
