use std::fmt::Debug;
use std::path::{Path, PathBuf};

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

/// Downloads the artifact `artifact_name` produced by the job into `dir`, returning its path.
///
/// The backend guarantees artifact names are plain filenames, so the file always lands in `dir`.
/// Overwrites any existing file with the same name.
pub async fn download(
    configuration: &Configuration,
    job_id: &str,
    artifact_name: &str,
    dir: &Path,
) -> anyhow::Result<PathBuf> {
    let ticket = default_api::issue_download_ticket(configuration, job_id, artifact_name)
        .await
        .map_err(error)?;

    let contents = configuration
        .client
        .get(&ticket.download_url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    let path = dir.join(artifact_name);
    std::fs::write(&path, contents)
        .with_context(|| format!("failed to write `{}`", path.display()))?;

    Ok(path)
}

/// Downloads all the artifacts the job produced into `dir` concurrently, creating `dir` if missing.
///
/// Returns the paths of the downloaded files, in the same order as `artifact_names`.
pub async fn download_all(
    configuration: &Configuration,
    job_id: &str,
    artifact_names: &[String],
    dir: &Path,
) -> anyhow::Result<Vec<PathBuf>> {
    std::fs::create_dir_all(dir)
        .with_context(|| format!("failed to create `{}`", dir.display()))?;

    futures_util::future::try_join_all(artifact_names.iter().map(|name| async move {
        download(configuration, job_id, name, dir)
            .await
            .with_context(|| format!("failed to download artifact `{name}`"))
    }))
    .await
}
