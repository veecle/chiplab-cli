use std::fmt::Debug;
use std::path::Path;
use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::ambient_authority;
use cap_std::fs_utf8::Dir;
use chiplab_api::apis::default_api;
use chiplab_api::apis::{Error, configuration::Configuration};
use futures_util::stream::{FuturesOrdered, Stream, TryStreamExt};
use tokio::io::AsyncWriteExt;

use crate::error::{BoxError, Result, ResultExt, WrappedError};

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
                WrappedError::new(
                    format_args!("server responded with {}", response.status),
                    content,
                )
                .into()
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
        std::fs::read(path).wrap_error(format_args!("failed to read `{}`", path.display()))?;

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

    let file = dir
        .create(artifact_name)
        .wrap_error(format_args!("failed to write `{artifact_name}`"))?;
    let mut file = tokio::fs::File::from_std(file.into_std());

    let stream = response.bytes_stream().map_err(std::io::Error::other);
    tokio::io::copy_buf(&mut tokio_util::io::StreamReader::new(stream), &mut file)
        .await
        .wrap_error(format_args!("failed to write `{artifact_name}`"))?;

    file.shutdown()
        .await
        .wrap_error(format_args!("failed to write `{artifact_name}`"))?;

    Ok(())
}

/// Prepares `dir` and returns a stream that downloads all the artifacts the job produced into it
/// concurrently.
///
/// The stream performs the downloads as it is polled, yielding the path of each file
/// along with whether it was downloaded, in the same order as `artifact_names`. Because multiple
/// downloads run at once, later files may finish downloading before earlier ones are yielded.
pub async fn download_all(
    configuration: &Configuration,
    job_id: &str,
    artifact_names: &[String],
    dir: &Utf8Path,
) -> Result<impl Stream<Item = (Utf8PathBuf, Result<()>)>> {
    Dir::create_ambient_dir_all(dir, ambient_authority())
        .wrap_error(format_args!("failed to open `{dir}`"))?;
    let handle = Arc::new(
        Dir::open_ambient_dir(dir, ambient_authority())
            .wrap_error(format_args!("failed to open `{dir}`"))?,
    );

    let stream = artifact_names
        .iter()
        .map(move |name| {
            let handle = handle.clone();
            async move {
                let result = download(configuration, job_id, name, &handle).await;
                (dir.join(name), result)
            }
        })
        .collect::<FuturesOrdered<_>>();

    Ok(stream)
}
