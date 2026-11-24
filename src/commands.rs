use crate::{
    api::Api,
    args::{Action, Options, Pattern},
    output,
};
use anyhow::{Result, ensure};
use reqwest::Method;
use serde_json::{Value, json};
use std::time::Duration;
use uuid::Uuid;

pub async fn run(options: Options) -> Result<u8> {
    let api = Api::new(&options.server)?;
    let value = match options.command {
        Action::Info { watch: true } => {
            return watch(&api, None, options.json, options.interval).await;
        }
        Action::Info { watch: false } => {
            let ready = api.get("readyz").await?;
            ensure!(ready["contract"] == 1, "Unsupported API contract");
            let mut system = api.get("v1/system").await?;
            system["ready"] = ready;
            system
        }
        Action::Pattern {
            command: Pattern::Preview { pattern },
        } => {
            api.request(
                Method::POST,
                "v1/patterns/preview",
                Some(json!({"pattern":pattern})),
            )
            .await?
        }
        Action::Submit(submission) => api.submit(&submission).await?,
        Action::Jobs => json!({"jobs":api.jobs().await?}),
        Action::Watch { id } => {
            return watch(
                &api,
                Some(api.resolve(&id).await?),
                options.json,
                options.interval,
            )
            .await;
        }
        Action::Status { id } => {
            api.get(&format!("v1/jobs/{}", api.resolve(&id).await?))
                .await?
        }
        Action::Stop { id } => mutation(&api, &id, "stop").await?,
        Action::Resume { id } => mutation(&api, &id, "resume").await?,
        Action::Result { id, output } => {
            let id = api.resolve(&id).await?;
            let result = api.get(&format!("v1/jobs/{id}/result")).await?;
            output::validate_result(&result, id)?;
            if let Some(path) = output {
                output::export(&result, &path)?;
            }
            result
        }
        Action::Delete { id, yes } => {
            ensure!(yes, "Deletion requires --yes");
            let id = api.resolve(&id).await?;
            api.request(Method::DELETE, &format!("v1/jobs/{id}"), None)
                .await?;
            json!({"deleted":id})
        }
    };
    output::print(&value, options.json)?;
    Ok(0)
}

async fn mutation(api: &Api, id: &str, action: &str) -> Result<Value> {
    api.request(
        Method::POST,
        &format!("v1/jobs/{}/{action}", api.resolve(id).await?),
        None,
    )
    .await
}

async fn watch(api: &Api, id: Option<Uuid>, json: bool, interval: f64) -> Result<u8> {
    tokio::select! {
        result = poll(api, id, json, Duration::from_secs_f64(interval)) => result,
        _ = tokio::signal::ctrl_c() => Ok(130),
    }
}

async fn poll(api: &Api, id: Option<Uuid>, json: bool, interval: Duration) -> Result<u8> {
    loop {
        let system = api.get("v1/system").await?;
        if let Some(id) = id {
            let job = api.get(&format!("v1/jobs/{id}")).await?;
            let exit = terminal(&job)?;
            output::print(&json!({"job":job,"system":system}), json)?;
            if let Some(exit) = exit {
                return Ok(exit);
            }
        } else {
            output::print(&system, json)?;
        }
        tokio::time::sleep(interval).await;
    }
}

fn terminal(job: &Value) -> Result<Option<u8>> {
    Ok(crate::contract::job(job)?.state.watch_exit())
}
