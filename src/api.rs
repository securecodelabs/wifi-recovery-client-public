use crate::{
    args::Submission,
    contract::{self, Search},
};
use anyhow::{Context, Result, ensure};
use reqwest::{Client, Method, Response, Url, multipart};
use serde_json::Value;
use std::time::Duration;
use uuid::Uuid;

pub struct Api {
    client: Client,
    root: Url,
}

impl Api {
    pub fn new(server: &str) -> Result<Self> {
        let root = Url::parse(server)?;
        ensure!(
            matches!(root.scheme(), "http" | "https") && root.host_str().is_some(),
            "Server must be an HTTP URL"
        );
        ensure!(
            root.username().is_empty() && root.password().is_none(),
            "Authentication is not supported"
        );
        ensure!(
            root.path() == "/" && root.query().is_none() && root.fragment().is_none(),
            "Server URL must not contain a path, query or fragment"
        );
        Ok(Self {
            root,
            client: Client::builder()
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(20))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
        })
    }

    pub async fn request(&self, method: Method, path: &str, body: Option<Value>) -> Result<Value> {
        let mut request = self.client.request(method, self.root.join(path)?);
        if let Some(body) = body {
            request = request.json(&body);
        }
        decode(request.send().await?).await
    }

    pub async fn get(&self, path: &str) -> Result<Value> {
        self.request(Method::GET, path, None).await
    }

    pub async fn job(&self, id: Uuid) -> Result<Value> {
        let value = self.get(&format!("v1/jobs/{id}")).await?;
        contract::job_for(&value, id)?;
        Ok(value)
    }

    pub async fn system(&self) -> Result<Value> {
        let value = self.get("v1/system").await?;
        contract::system(&value)?;
        Ok(value)
    }

    pub async fn submit(&self, submission: &Submission) -> Result<Value> {
        let file = tokio::fs::File::open(&submission.file).await?;
        let metadata = file.metadata().await?;
        let length = metadata.len();
        ensure!(
            metadata.is_file() && length > 0 && length <= 8 * 1024 * 1024,
            "Input must be a regular file containing 1 byte to 8 MiB (server may impose a lower limit)"
        );
        let filename = submission
            .file
            .file_name()
            .context("Missing filename")?
            .to_string_lossy()
            .into_owned();
        let stream = tokio_util::io::ReaderStream::new(file);
        let part = multipart::Part::stream_with_length(reqwest::Body::wrap_stream(stream), length)
            .file_name(filename)
            .mime_str("application/octet-stream")?;
        let form = multipart::Form::new()
            .text(
                "metadata",
                serde_json::to_string(&Search {
                    pattern: submission.pattern.clone(),
                    runtime_seconds: submission.runtime,
                    candidate_limit: submission.limit,
                })?,
            )
            .part("file", part);
        let mut request = self
            .client
            .post(self.root.join("v1/jobs")?)
            .timeout(Duration::from_secs(90))
            .multipart(form);
        if let Some(key) = submission.request_id {
            request = request.header("Idempotency-Key", key.to_string());
        }
        let value = decode(request.send().await?).await?;
        contract::job(&value)?;
        Ok(value)
    }

    pub async fn jobs(&self) -> Result<Vec<Value>> {
        let mut jobs = Vec::new();
        loop {
            let page = self
                .get(&format!("v1/jobs?limit=100&offset={}", jobs.len()))
                .await?;
            let batch = page["jobs"].as_array().context("Invalid job list")?;
            let total = page["total"].as_u64().context("Missing job count")?;
            ensure!(total <= 10000, "Server list exceeds supported capacity");
            jobs.extend_from_slice(batch);
            if jobs.len() as u64 >= total || batch.is_empty() {
                return Ok(jobs);
            }
        }
    }

    pub async fn resolve(&self, id: &str) -> Result<Uuid> {
        if let Ok(id) = Uuid::parse_str(id) {
            return Ok(id);
        }
        ensure!(
            id.len() >= 4
                && id.len() < 36
                && id.bytes().all(|c| c.is_ascii_hexdigit() || c == b'-'),
            "Expected a UUID or prefix of at least four characters"
        );
        let prefix = id.to_ascii_lowercase();
        let ids: Vec<Uuid> = self
            .jobs()
            .await?
            .iter()
            .filter_map(|job| job["id"].as_str())
            .filter(|id| id.starts_with(&prefix))
            .map(str::parse)
            .collect::<Result<_, _>>()?;
        ensure!(
            ids.len() == 1,
            "Job prefix matches {} jobs; use a full UUID",
            ids.len()
        );
        Ok(ids[0])
    }
}

async fn decode(mut response: Response) -> Result<Value> {
    let status = response.status();
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        ensure!(
            bytes.len() + chunk.len() <= 32 * 1024 * 1024,
            "API response exceeded 32 MiB"
        );
        bytes.extend_from_slice(&chunk);
    }
    if !status.is_success() {
        return Err(response_error(status, &bytes));
    }
    if status == reqwest::StatusCode::NO_CONTENT {
        return Ok(Value::Null);
    }
    let value: Value = serde_json::from_slice(&bytes).context("API returned invalid JSON")?;
    contract::validate_jobs(&value)?;
    Ok(value)
}

fn response_error(status: reqwest::StatusCode, bytes: &[u8]) -> anyhow::Error {
    let value = serde_json::from_slice::<Value>(bytes).ok();
    let text = String::from_utf8_lossy(bytes);
    let message = value
        .as_ref()
        .and_then(|v| v["error"]["message"].as_str())
        .unwrap_or(text.trim());
    let message = if message.is_empty() {
        status.canonical_reason().unwrap_or("Request failed")
    } else {
        message
    };
    anyhow::anyhow!(
        "HTTP {}: {}",
        status.as_u16(),
        message.chars().take(2048).collect::<String>()
    )
}
