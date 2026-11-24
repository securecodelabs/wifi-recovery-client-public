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

    pub async fn submit(&self, submission: &Submission) -> Result<Value> {
        let file = tokio::fs::File::open(&submission.file).await?;
        let length = file.metadata().await?.len();
        ensure!(
            length > 0 && length <= 64 * 1024 * 1024,
            "Input must contain 1 byte to 64 MiB (server may impose a lower limit)"
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
        decode(request.send().await?).await
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
    if status == reqwest::StatusCode::NO_CONTENT {
        return Ok(Value::Null);
    }
    let value: Value = serde_json::from_slice(&bytes).context("API returned invalid JSON")?;
    ensure!(
        status.is_success(),
        "HTTP {}: {}",
        status.as_u16(),
        value["error"]["message"]
            .as_str()
            .unwrap_or("Request failed")
    );
    contract::validate_jobs(&value)?;
    Ok(value)
}
