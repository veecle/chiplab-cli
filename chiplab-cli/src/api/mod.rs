use std::fmt::Debug;
use std::path::Path;

use anyhow::Context;
use chiplab_api::apis::default_api;
use chiplab_api::apis::{Error, configuration::Configuration};

pub mod models;

/// Converts an API error into one that includes the response body the server sent.
pub fn error<T>(error: Error<T>) -> anyhow::Error
where
    T: Debug + Send + Sync + 'static,
{
    match error {
        Error::ResponseError(response) => {
            let content = response.content.trim();
            if content.is_empty() {
                anyhow::anyhow!("server responded with {}", response.status)
            } else {
                anyhow::anyhow!("server responded with {}: {content}", response.status)
            }
        }
        error => anyhow::Error::new(error),
    }
}

/// Uploads the file at `path` as a new artifact in the project, returning its id.
pub async fn upload(
    configuration: &Configuration,
    project_id: &str,
    path: &Path,
) -> anyhow::Result<String> {
    let contents =
        std::fs::read(path).with_context(|| format!("failed to read `{}`", path.display()))?;

    let ticket = default_api::issue_upload_ticket(configuration, project_id)
        .await
        .map_err(error)?;

    let response = configuration
        .client
        .put(&ticket.upload_url)
        .body(contents)
        .send()
        .await
        .context("failed to send the upload")?;
    if !response.status().is_success() {
        anyhow::bail!("upload server responded with {}", response.status());
    }

    Ok(ticket.artifact_id)
}
