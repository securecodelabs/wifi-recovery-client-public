use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct Search {
    pub pattern: String,
    pub runtime_seconds: u64,
    pub candidate_limit: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    Stopping,
    Stopped,
    Interrupted,
    LimitReached,
    Recovered,
    Exhausted,
    Failed,
}

impl JobState {
    pub fn watch_exit(&self) -> Option<u8> {
        match self {
            Self::Queued | Self::Running | Self::Stopping => None,
            Self::Recovered => Some(0),
            Self::Exhausted => Some(10),
            Self::Stopped => Some(11),
            Self::LimitReached => Some(12),
            Self::Failed => Some(13),
            Self::Interrupted => Some(14),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct JobReply {
    pub id: Uuid,
    pub state: JobState,
    pub search: Search,
    pub gpu_batch: u32,
    pub filename: String,
    pub input_sha256: String,
    pub input_bytes: u64,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub first_started_at_ms: Option<u64>,
    pub total_active_seconds: f64,
    pub attempts: Vec<Attempt>,
    pub progress: Progress,
    pub error: Option<String>,
    pub resume_available: bool,
    pub resume_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Attempt {
    pub number: u32,
    pub started_at_ms: u64,
    pub finished_at_ms: Option<u64>,
    pub search_started_at_ms: Option<u64>,
    pub elapsed_seconds: f64,
    pub timing_complete: bool,
    pub outcome: JobState,
    pub engine_run_at_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Progress {
    pub stage: String,
    pub fresh: bool,
    pub sample_at_ms: Option<u64>,
    pub rate_sample_at_ms: Option<u64>,
    pub estimate_sample_at_ms: Option<u64>,
    pub tested_candidates: Option<u64>,
    pub total_candidates: Option<u64>,
    pub fraction: Option<f64>,
    pub recovered_records: Option<u64>,
    pub total_records: Option<u64>,
    pub candidate_hex: Option<String>,
    pub candidate_text: Option<String>,
    pub candidates_per_second: Option<f64>,
    pub kh_per_second: Option<f64>,
    pub speed_basis: String,
    pub search_elapsed_seconds: Option<f64>,
    pub estimated_full_seconds: Option<f64>,
    pub search_eta_seconds: Option<f64>,
    pub estimated_finish_at_ms: Option<u64>,
    pub runtime_remaining_seconds: Option<f64>,
    pub candidate_limit_remaining: Option<u64>,
}

pub fn job(value: &serde_json::Value) -> anyhow::Result<JobReply> {
    Ok(serde_json::from_value(value.clone())?)
}

pub fn job_for(value: &serde_json::Value, id: Uuid) -> anyhow::Result<JobReply> {
    let reply = job(value)?;
    anyhow::ensure!(reply.id == id, "API returned a different job");
    Ok(reply)
}

pub fn validate_jobs(value: &serde_json::Value) -> anyhow::Result<()> {
    if value.get("id").is_some() {
        job(value)?;
    }
    if let Some(jobs) = value.get("jobs") {
        let jobs = jobs
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("API jobs field is not an array"))?;
        for entry in jobs {
            job(entry)?;
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct SystemReply {
    pub contract: u32,
    pub sample_age_ms: u64,
    pub source: String,
    pub resources: Resources,
}

#[derive(Debug, Deserialize)]
pub struct Resources {
    pub sampled_at_ms: u64,
    pub cpu_percent: Option<f64>,
    pub memory_total_bytes: Option<u64>,
    pub memory_available_bytes: Option<u64>,
    pub memory_used_bytes: Option<u64>,
    pub gpus: Vec<Gpu>,
    pub selected_gpu_pci: Option<String>,
    pub gpu_mapping: String,
}

#[derive(Debug, Deserialize)]
pub struct Gpu {
    pub pci: String,
    pub utilization_percent: Option<u64>,
    pub memory_used_bytes: Option<u64>,
    pub memory_total_bytes: Option<u64>,
}

pub fn system(value: &serde_json::Value) -> anyhow::Result<SystemReply> {
    let reply: SystemReply = serde_json::from_value(value.clone())?;
    anyhow::ensure!(reply.contract == 1, "Unsupported API contract");
    Ok(reply)
}
