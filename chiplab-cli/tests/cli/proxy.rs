//! A record/replay proxy standing in for the Chiplab API during fixture tests.
//!
//! The mode is selected by `TEST_CHIPLAB_PROXY`, see [`Mode`].
//!
//! In replay mode the proxy serves recorded exchanges in order, failing on any request that doesn't
//! match the next recorded one. In record mode it forwards requests to a real Chiplab instance and
//! records the exchanges. In bypass mode it forwards requests like in record mode, but neither
//! replays nor records exchanges, for running the cases against a real Chiplab instance.
//!
//! Absolute URLs to other hosts in JSON responses are rewritten to point back at the proxy, so
//! requests the CLI makes to them (such as uploads) are recorded and replayed too. Request headers
//! are not recorded; the proxy authenticates to the real instance itself so no credentials end up
//! in the recordings.
//!
//! While recording, the CLI only sees fake TypeIDs: the proxy swaps them for the real ones in
//! requests, and back in responses. That keeps recordings and the CLI output stable across
//! re-recordings, and lets cases refer to a project as [`PROJECT_ID`] whatever project is used to
//! record them.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, LazyLock};
use std::thread::JoinHandle;

use regex::Regex;

use crate::error::{BoxError, Chain, Result, WrapError};

/// Stands in for the proxy's own URL in recorded bodies, as its port differs between runs.
const BASE_URL_PLACEHOLDER: &str = "{{base_url}}";

/// The path prefix under which the proxy serves rewritten URLs to other hosts.
const EXTERNAL_PREFIX: &str = "/external/";

const DEFAULT_URL: &str = "https://chiplab.veecle.ai";

/// One request to the proxy and the response it was answered with.
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Exchange {
    request: Request,
    response: Response,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
struct Request {
    method: String,

    /// The path including any query string.
    path: String,

    /// The body, only recorded for JSON requests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    body: Option<serde_json::Value>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct Response {
    status: u16,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    body: Option<Body>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
enum Body {
    Json(serde_json::Value),
    Text(String),
}

/// Reads the exchanges recorded at `path`, a missing file means no exchanges.
fn load(path: &Path) -> Result<Vec<Exchange>> {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            Err(error).wrap_error_with(|| format!("failed to read `{}`", path.display()))?
        }
    };

    Ok(serde_json::from_str(&contents)
        .wrap_error_with(|| format!("failed to parse `{}`", path.display()))?)
}

/// Writes `exchanges` to `path`, removing the file instead if there are none.
fn save(path: &Path, exchanges: &[Exchange]) -> Result<()> {
    if exchanges.is_empty() {
        match std::fs::remove_file(path) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                Err(error).wrap_error_with(|| format!("failed to remove `{}`", path.display()))?
            }
            _ => {}
        }
        return Ok(());
    }
    let mut contents = serde_json::to_string_pretty(exchanges)?;
    contents.push('\n');
    std::fs::write(path, contents)
        .wrap_error_with(|| format!("failed to write `{}`", path.display()))?;
    Ok(())
}

/// How the proxy answers requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Serve the recorded exchanges in order.
    #[default]
    Replay,

    /// Forward to a real Chiplab instance and record the exchanges.
    Record,

    /// Forward to a real Chiplab instance, ignoring the recorded exchanges.
    Bypass,
}

impl Mode {
    /// Reads the mode from `TEST_CHIPLAB_PROXY`, defaulting to [`Mode::Replay`].
    pub fn from_env() -> Result<Self> {
        let mode = match std::env::var("TEST_CHIPLAB_PROXY") {
            Ok(mode) => mode,
            Err(std::env::VarError::NotPresent) => return Ok(Self::default()),
            Err(error) => Err(error).wrap_error("failed to read `TEST_CHIPLAB_PROXY`")?,
        };

        Ok(mode
            .parse()
            .wrap_error("failed to parse `TEST_CHIPLAB_PROXY`")?)
    }
}

impl FromStr for Mode {
    type Err = BoxError;

    fn from_str(mode: &str) -> Result<Self> {
        match mode {
            "replay" => Ok(Self::Replay),
            "record" => Ok(Self::Record),
            "bypass" => Ok(Self::Bypass),
            _ => Err(
                format!("unknown mode `{mode}`, expected `replay`, `record` or `bypass`").into(),
            ),
        }
    }
}

/// Where the proxy gets its responses from.
enum Backend {
    /// Serve these exchanges in order.
    Replay(Vec<Exchange>),

    /// Forward to a real Chiplab instance, recording the exchanges if `record` is set.
    Forward {
        upstream: Box<Upstream>,
        record: bool,
    },
}

/// The real Chiplab instance to record exchanges with.
struct Upstream {
    url: String,
    token: String,
    client: reqwest::blocking::Client,

    /// The original URLs behind `/external/<index>`.
    external: Vec<String>,

    ids: Ids,
}

impl Upstream {
    /// Configures the instance from `CHIPLAB_URL` and `CHIPLAB_TOKEN`, as the CLI itself does.
    ///
    /// `TEST_CHIPLAB_PROJECT_ID` optionally sets the real project behind [`PROJECT_ID`].
    fn from_env() -> Result<Self> {
        let url = match std::env::var("CHIPLAB_URL") {
            Ok(url) => url,
            Err(std::env::VarError::NotPresent) => DEFAULT_URL.to_owned(),
            Err(error) => Err(error).wrap_error("failed to read `CHIPLAB_URL`")?,
        };
        let token = std::env::var("CHIPLAB_TOKEN").wrap_error("failed to read `CHIPLAB_TOKEN`")?;

        let mut ids = Ids::default();
        match std::env::var("TEST_CHIPLAB_PROJECT_ID") {
            Ok(project_id) => {
                let fake_id = ids.normalize(&project_id);
                if fake_id != PROJECT_ID {
                    return Err(format!(
                        "`TEST_CHIPLAB_PROJECT_ID` must be a project TypeID, got `{project_id}`"
                    )
                    .into());
                }
            }
            Err(std::env::VarError::NotPresent) => {}
            Err(error) => Err(error).wrap_error("failed to read `TEST_CHIPLAB_PROJECT_ID`")?,
        }

        let user_agent = concat!(
            env!("CARGO_PKG_NAME"),
            "-test-proxy/",
            env!("CARGO_PKG_VERSION"),
            " ",
            "(",
            env!("CARGO_PKG_REPOSITORY"),
            ")",
        );
        // Waiting for jobs uses long-polling, so there must be a generous request timeout.
        let client = reqwest::blocking::Client::builder()
            .user_agent(user_agent)
            .timeout(std::time::Duration::from_secs(5 * 60))
            .build()
            .wrap_error("failed to create the HTTP client")?;
        Ok(Self {
            url: url.trim_end_matches('/').to_owned(),
            token,
            client,
            external: Vec::new(),
            ids,
        })
    }

    /// Forwards `request` with fake ids replaced by the real ones, and returns the response with
    /// real ids replaced by fake ones and absolute URLs to other hosts rewritten to point back at
    /// the proxy.
    fn forward(
        &mut self,
        request: &Request,
        content_type: Option<&str>,
        body: Vec<u8>,
    ) -> Result<Response> {
        let method = reqwest::Method::from_bytes(request.method.as_bytes())?;
        let mut builder = match request.path.strip_prefix(EXTERNAL_PREFIX) {
            Some(index) => {
                let url = index
                    .parse::<usize>()
                    .ok()
                    .and_then(|index| self.external.get(index))
                    .ok_or_else(|| format!("unknown external URL `{}`", request.path))?;
                self.client.request(method, url)
            }
            None => {
                let path = self.ids.denormalize(&request.path);
                self.client
                    .request(method, format!("{}{path}", self.url))
                    .bearer_auth(&self.token)
            }
        };
        let body = match content_type {
            Some(content_type) => {
                builder = builder.header(reqwest::header::CONTENT_TYPE, content_type);
                if is_json(content_type) {
                    let body =
                        std::str::from_utf8(&body).wrap_error("invalid JSON request body")?;
                    self.ids.denormalize(body).into_bytes()
                } else {
                    body
                }
            }
            None => body,
        };

        let response = builder.body(body).send()?;
        let status = response.status().as_u16();
        let is_json = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(is_json);
        let text = response.text()?;

        let body = if text.is_empty() {
            None
        } else if is_json {
            let mut value = serde_json::from_str(&text)?;
            // URLs are externalized first so the original URLs keep their real ids.
            for_each_string(&mut value, &mut |string| {
                let is_url = string.starts_with("http://") || string.starts_with("https://");
                // TODO: For now skip any URLs that point to Chiplab itself, there are not any used
                // currently and it's not known whether these will be either requested or written to
                // the output which would require different handling.
                if is_url && !string.starts_with(&self.url) {
                    let index = self.external.len();
                    let url = std::mem::replace(
                        string,
                        format!("{BASE_URL_PLACEHOLDER}{EXTERNAL_PREFIX}{index}"),
                    );
                    self.external.push(url);
                }
            });
            for_each_string(&mut value, &mut |string| {
                *string = self.ids.normalize(string);
            });
            Some(Body::Json(value))
        } else {
            Some(Body::Text(self.ids.normalize(&text)))
        };
        Ok(Response { status, body })
    }
}

/// The fake id standing in for `TEST_CHIPLAB_PROJECT_ID` in recordings.
const PROJECT_ID: &str = "project_00000000000000000000000001";

/// Matches a [TypeID](https://github.com/jetify-com/typeid/tree/main/spec).
static TYPE_ID: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b([a-z](?:[a-z_]{0,61}[a-z])?)_[0-7][0-9a-hjkmnp-tv-z]{25}\b")
        .expect("the TypeID pattern is valid")
});

/// Maps the real TypeIDs seen while recording to fake ones, numbered per prefix in order of first
/// appearance, so recordings don't depend on the ids a recording session happened to create.
#[derive(Default)]
struct Ids {
    to_fake: HashMap<String, String>,
    to_real: HashMap<String, String>,
    counts: HashMap<String, u64>,
}

impl Ids {
    /// Replaces the real TypeIDs in `text` with fake ones, assigning new fake ids as needed.
    fn normalize(&mut self, text: &str) -> String {
        TYPE_ID
            .replace_all(text, |captures: &regex::Captures<'_>| {
                let real = &captures[0];
                if let Some(fake) = self.to_fake.get(real) {
                    return fake.clone();
                }
                let prefix = &captures[1];
                let count = self.counts.entry(prefix.to_owned()).or_default();
                *count += 1;
                let fake = format!("{prefix}_{count:026}");
                self.to_fake.insert(real.to_owned(), fake.clone());
                self.to_real.insert(fake.clone(), real.to_owned());
                fake
            })
            .into_owned()
    }

    /// Replaces the fake TypeIDs in `text` with the real ones they stand in for.
    ///
    /// Unknown ids, such as ones a case passes in its arguments, are passed through unchanged and
    /// mapped to themselves, so they stay the same if the server echoes them back. Such ids must
    /// not collide with the generated fakes, so use either the all-zero id or one that isn't a
    /// small number.
    fn denormalize(&mut self, text: &str) -> String {
        TYPE_ID
            .replace_all(text, |captures: &regex::Captures<'_>| {
                let fake = &captures[0];
                if let Some(real) = self.to_real.get(fake) {
                    return real.clone();
                }
                self.to_fake.insert(fake.to_owned(), fake.to_owned());
                self.to_real.insert(fake.to_owned(), fake.to_owned());
                fake.to_owned()
            })
            .into_owned()
    }
}

/// Calls `visit` on every string within `value`.
fn for_each_string(value: &mut serde_json::Value, visit: &mut impl FnMut(&mut String)) {
    match value {
        serde_json::Value::String(string) => visit(string),
        serde_json::Value::Array(values) => {
            for value in values {
                for_each_string(value, visit);
            }
        }
        serde_json::Value::Object(map) => {
            for value in map.values_mut() {
                for_each_string(value, visit);
            }
        }
        _ => {}
    }
}

fn is_json(content_type: &str) -> bool {
    content_type.starts_with("application/json")
}

/// A running proxy, listening on a local port.
pub struct Proxy {
    server: Arc<tiny_http::Server>,
    base_url: String,
    thread: JoinHandle<State>,

    /// Where the exchanges are recorded.
    exchanges_path: PathBuf,
}

impl Proxy {
    /// Starts a proxy replaying the exchanges at `exchanges_path`, recording to it, or bypassing it.
    pub fn start(mode: Mode, exchanges_path: &Path) -> Result<Self> {
        let backend = match mode {
            Mode::Replay => Backend::Replay(load(exchanges_path)?),
            Mode::Record | Mode::Bypass => Backend::Forward {
                upstream: Box::new(Upstream::from_env()?),
                record: mode == Mode::Record,
            },
        };

        let server =
            tiny_http::Server::http("127.0.0.1:0").wrap_error("failed to start the proxy")?;
        let server = Arc::new(server);
        let port = server
            .server_addr()
            .to_ip()
            .ok_or("the proxy doesn't listen on an IP address")?
            .port();
        let base_url = format!("http://127.0.0.1:{port}");

        let mut state = State {
            backend,
            base_url: base_url.clone(),
            next: 0,
            recorded: Vec::new(),
            errors: Vec::new(),
        };
        let thread = std::thread::spawn({
            let server = server.clone();
            move || {
                for request in server.incoming_requests() {
                    state.handle(request);
                }
                state
            }
        });

        Ok(Self {
            server,
            base_url,
            thread,
            exchanges_path: exchanges_path.to_owned(),
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Stops the proxy, saving the recorded exchanges in record mode.
    ///
    /// Fails if any request didn't match the recording, or if recorded exchanges weren't requested.
    pub fn finish(self) -> Result<()> {
        self.server.unblock();
        let state = self
            .thread
            .join()
            .map_err(|_| "the proxy thread panicked")?;

        if !state.errors.is_empty() {
            return Err(format!("proxy errors:\n{}", state.errors.join("\n")).into());
        }
        match state.backend {
            Backend::Replay(exchanges) => {
                if let Some(exchange) = exchanges.get(state.next) {
                    return Err(format!(
                        "{} recorded exchanges were not requested, starting with `{} {}`",
                        exchanges.len() - state.next,
                        exchange.request.method,
                        exchange.request.path,
                    )
                    .into());
                }
                Ok(())
            }
            Backend::Forward { record: true, .. } => save(&self.exchanges_path, &state.recorded),
            Backend::Forward { record: false, .. } => Ok(()),
        }
    }
}

struct State {
    backend: Backend,
    base_url: String,

    /// The index of the next exchange to replay.
    next: usize,

    recorded: Vec<Exchange>,

    errors: Vec<String>,
}

impl State {
    fn handle(&mut self, mut request: tiny_http::Request) {
        let response = self.exchange(&mut request).unwrap_or_else(|error| {
            let message = Chain(&*error).to_string();
            self.errors.push(message.clone());
            Response {
                status: 500,
                body: Some(Body::Text(message)),
            }
        });
        // The CLI hanging up early shows up in its own output.
        let _ = request.respond(self.http_response(&response));
    }

    fn exchange(&mut self, request: &mut tiny_http::Request) -> Result<Response> {
        let mut body = Vec::new();
        request
            .as_reader()
            .read_to_end(&mut body)
            .wrap_error("failed to read the request body")?;
        let content_type = request
            .headers()
            .iter()
            .find(|header| header.field.equiv("Content-Type"))
            .map(|header| header.value.as_str().to_owned());
        let json_body = match content_type.as_deref() {
            Some(content_type) if is_json(content_type) => Some(
                serde_json::from_slice(&body)
                    .wrap_error("failed to parse the JSON request body")?,
            ),
            _ => None,
        };
        let actual = Request {
            method: request.method().as_str().to_owned(),
            path: request.url().to_owned(),
            body: json_body,
        };

        match &mut self.backend {
            Backend::Replay(exchanges) => {
                let exchange = exchanges.get(self.next).ok_or_else(|| {
                    format!(
                        "unexpected request `{} {}` after all recorded exchanges",
                        actual.method, actual.path
                    )
                })?;
                if exchange.request != actual {
                    return Err(format!(
                        "request {actual:?} doesn't match recorded request {:?}",
                        exchange.request,
                    )
                    .into());
                }
                self.next += 1;
                Ok(exchange.response.clone())
            }
            Backend::Forward { upstream, record } => {
                let response = upstream
                    .forward(&actual, content_type.as_deref(), body)
                    .wrap_error_with(|| {
                        format!("failed to forward `{} {}`", actual.method, actual.path)
                    })?;
                if *record {
                    self.recorded.push(Exchange {
                        request: actual,
                        response: response.clone(),
                    });
                }
                Ok(response)
            }
        }
    }

    fn http_response(&self, response: &Response) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
        let (content_type, data) = match &response.body {
            None => (None, String::new()),
            Some(Body::Json(value)) => (Some("application/json"), value.to_string()),
            Some(Body::Text(text)) => (Some("text/plain"), text.clone()),
        };
        let data = data.replace(BASE_URL_PLACEHOLDER, &self.base_url);

        let mut http_response =
            tiny_http::Response::from_data(data.into_bytes()).with_status_code(response.status);
        if let Some(content_type) = content_type {
            let header = tiny_http::Header::from_bytes("Content-Type", content_type)
                .expect("the content type is a valid header");
            http_response = http_response.with_header(header);
        }
        http_response
    }
}
