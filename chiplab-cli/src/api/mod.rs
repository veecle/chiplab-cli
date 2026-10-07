use std::fmt::Debug;
use std::path::Path;

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::ambient_authority;
use cap_std::fs_utf8::Dir;
use chiplab_api::apis::default_api;
use chiplab_api::apis::{Error, configuration::Configuration};
use futures_util::TryStreamExt;
use tokio::io::AsyncWriteExt;

use crate::error::{BoxError, Result, WrapError};

pub mod models;

/// Converts an API error into one that includes the response body the server sent.
pub fn error<T>(error: Error<T>) -> BoxError
where
    T: Debug + Send + Sync + 'static,
{
    match error {
        Error::ResponseError(response) => {
            let content = response.content.trim();
            if content.is_empty() {
                format!("server responded with {}", response.status).into()
            } else {
                format!("server responded with {}: {content}", response.status).into()
            }
        }
        error => error.into(),
    }
}

/// Uploads the file at `path` as a new artifact in the project, returning its id.
pub async fn upload(
    configuration: &Configuration,
    project_id: &str,
    path: &Path,
) -> Result<String> {
    let contents =
        std::fs::read(path).wrap_error_with(|| format!("failed to read `{}`", path.display()))?;

    let ticket = default_api::issue_upload_ticket(configuration, project_id)
        .await
        .map_err(error)?;

    let response = configuration
        .client
        .put(&ticket.upload_url)
        .body(contents)
        .send()
        .await
        .wrap_error("failed to send the upload")?;
    if !response.status().is_success() {
        return Err(format!("upload server responded with {}", response.status()).into());
    }

    Ok(ticket.artifact_id)
}

/// Downloads the artifact `artifact_name` produced by the job into `dir`.
///
/// Fails if `artifact_name` resolves outside `dir`. Overwrites any existing file with the same name.
pub async fn download(
    configuration: &Configuration,
    job_id: &str,
    artifact_name: &str,
    dir: &Dir,
) -> Result<()> {
    let ticket = default_api::issue_download_ticket(configuration, job_id, artifact_name)
        .await
        .map_err(error)?;

    let response = configuration
        .client
        .get(&ticket.download_url)
        .send()
        .await?
        .error_for_status()?;

    let write_context = || format!("failed to write `{artifact_name}`");

    let file = dir.create(artifact_name).wrap_error_with(write_context)?;
    let mut file = tokio::fs::File::from_std(file.into_std());

    let stream = response.bytes_stream().map_err(std::io::Error::other);
    tokio::io::copy_buf(&mut tokio_util::io::StreamReader::new(stream), &mut file)
        .await
        .wrap_error_with(write_context)?;

    file.shutdown().await.wrap_error_with(write_context)?;

    Ok(())
}

/// Downloads all the artifacts the job produced into `dir` concurrently, creating `dir` if missing.
///
/// Returns the paths of the downloaded files, in the same order as `artifact_names`.
pub async fn download_all(
    configuration: &Configuration,
    job_id: &str,
    artifact_names: &[String],
    dir: &Utf8Path,
) -> Result<Vec<Utf8PathBuf>> {
    let open_context = || format!("failed to open `{dir}`");

    Dir::create_ambient_dir_all(dir, ambient_authority()).wrap_error_with(open_context)?;
    let handle = Dir::open_ambient_dir(dir, ambient_authority()).wrap_error_with(open_context)?;

    futures_util::future::try_join_all(artifact_names.iter().map(|name| {
        let handle = &handle;
        async move {
            download(configuration, job_id, name, handle)
                .await
                .wrap_error_with(|| format!("failed to download artifact `{name}`"))?;

            Ok(dir.join(name))
        }
    }))
    .await
}
