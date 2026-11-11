//! A record/replay proxy standing in for the Chiplab API during fixture tests.
//!
//! The modes and what is recorded are documented in the "Recording API exchanges" section of
//! `CONTRIBUTING.md`; keep it up to date when changing them.

use std::borrow::Borrow;
use std::collections::HashMap;
use std::convert::Infallible;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, LazyLock};
use std::thread::JoinHandle;

use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper_util::rt::TokioIo;
use regex::Regex;
use tokio::sync::{Mutex, oneshot};

use crate::error::{BoxError, Chain, Result, ResultExt};
use crate::temp_dir;

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

impl Request {
    /// A representation of the request that is stable between runs, so that what is derived from
    /// it doesn't depend on the order concurrent requests are made in.
    fn stable_key(&self) -> (String, String, Option<String>) {
        (
            self.path.clone(),
            self.method.clone(),
            self.body.as_ref().map(ToString::to_string),
        )
    }

    fn stable_hash(&self) -> RequestHash {
        let data = serde_json::to_vec(&self.stable_key()).expect("keys can be serialized");
        // 64-bit FNV-1a.
        RequestHash(data.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        }))
    }
}

/// A hash of a request's [`Request::stable_key`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct RequestHash(u64);

impl std::fmt::Display for RequestHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

/// Identifies an external URL by the request whose response contained it, and its index among the
/// external URLs in the responses to that request.
///
/// The proxy serves the URL at [`EXTERNAL_PREFIX`] followed by the key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ExternalKey {
    request: RequestHash,
    index: u64,
}

impl ExternalKey {
    fn parse(key: &str) -> Option<Self> {
        let (request, index) = key.split_once('/')?;
        Some(Self {
            request: RequestHash(u64::from_str_radix(request, 16).ok()?),
            index: index.parse().ok()?,
        })
    }
}

impl std::fmt::Display for ExternalKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.request, self.index)
    }
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
        Err(error) => Err(error).wrap_error(format_args!("failed to read `{}`", path.display()))?,
    };

    Ok(serde_json::from_str(&contents)
        .wrap_error(format_args!("failed to parse `{}`", path.display()))?)
}

/// Writes `exchanges` to `path`, removing the file instead if there are none.
fn save(path: &Path, exchanges: &[Exchange]) -> Result<()> {
    if exchanges.is_empty() {
        match std::fs::remove_file(path) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                Err(error).wrap_error(format_args!("failed to remove `{}`", path.display()))?
            }
            _ => {}
        }
        return Ok(());
    }
    let mut contents = serde_json::to_string_pretty(exchanges)?;
    contents.push('\n');
    std::fs::write(path, contents)
        .wrap_error(format_args!("failed to write `{}`", path.display()))?;
    Ok(())
}

/// Orders `exchanges` by request independently of the order the requests came in, as the CLI
/// makes some concurrently, keeping identical requests in their recorded order.
fn sort(exchanges: &mut [Exchange]) {
    exchanges.sort_by_cached_key(|exchange| exchange.request.stable_key());
}

/// How the proxy answers requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Serve the recorded exchanges.
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
    /// Serve these exchanges, removing each once it's served.
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
    client: reqwest::Client,

    /// The original URLs behind the external URLs.
    external: HashMap<ExternalKey, String>,

    /// How many external URLs the responses to each request have contained so far.
    external_counts: HashMap<RequestHash, u64>,

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
                let is_project_id = TYPE_ID
                    .captures(&project_id)
                    .is_some_and(|captures| captures[0] == project_id && &captures[1] == "project");
                if !is_project_id {
                    return Err(format!(
                        "`TEST_CHIPLAB_PROJECT_ID` must be a project TypeID, got `{project_id}`"
                    )
                    .into());
                }
                ids.insert(RealId(project_id), FakeId(PROJECT_ID.to_owned()));
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
        let client = reqwest::Client::builder()
            .user_agent(user_agent)
            .timeout(std::time::Duration::from_secs(5 * 60))
            .build()
            .wrap_error("failed to create the HTTP client")?;
        Ok(Self {
            url: url.trim_end_matches('/').to_owned(),
            token,
            client,
            external: HashMap::new(),
            external_counts: HashMap::new(),
            ids,
        })
    }

    /// Forwards `request` with fake ids replaced by the real ones, and returns the response with
    /// real ids replaced by fake ones and absolute URLs to other hosts rewritten to point back at
    /// the proxy.
    async fn forward(
        &mut self,
        request: &Request,
        content_type: Option<&str>,
        body: Vec<u8>,
    ) -> Result<Response> {
        let hash = request.stable_hash();
        let method = reqwest::Method::from_bytes(request.method.as_bytes())?;
        let mut builder = match request.path.strip_prefix(EXTERNAL_PREFIX) {
            Some(key) => {
                let url = ExternalKey::parse(key)
                    .and_then(|key| self.external.get(&key))
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

        let response = builder.body(body).send().await?;
        let status = response.status().as_u16();
        let is_json = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(is_json);
        let text = response.text().await?;

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
                    let count = self.external_counts.entry(hash).or_default();
                    let key = ExternalKey {
                        request: hash,
                        index: *count,
                    };
                    *count += 1;
                    let url = std::mem::replace(
                        string,
                        format!("{BASE_URL_PLACEHOLDER}{EXTERNAL_PREFIX}{key}"),
                    );
                    self.external.insert(key, url);
                }
            });
            for_each_string(&mut value, &mut |string| {
                *string = temp_dir::redact(&self.ids.normalize(string, hash));
            });
            Some(Body::Json(value))
        } else {
            Some(Body::Text(temp_dir::redact(
                &self.ids.normalize(&text, hash),
            )))
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

/// Maps the real TypeIDs seen while recording to fake ones, so recordings don't depend on the ids a
/// recording session happened to create.
///
/// A fake id is made of a hash of the request whose response it first appeared in, and a count of
/// the ids first appearing in responses to that request.
///
/// The CLI only sees the fake ids, which are swapped back for the real ones in requests.
#[derive(Default)]
struct Ids {
    to_fake: HashMap<RealId, FakeId>,
    to_real: HashMap<FakeId, RealId>,

    /// How many fake ids have been assigned for the responses to each request so far.
    counts: HashMap<RequestHash, u64>,
}

/// A TypeID as Chiplab knows it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RealId(String);

/// A TypeID standing in for a [`RealId`] in recordings and towards the CLI.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FakeId(String);

impl Borrow<str> for RealId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for FakeId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl Ids {
    fn insert(&mut self, real: RealId, fake: FakeId) {
        self.to_fake.insert(real.clone(), fake.clone());
        self.to_real.insert(fake, real);
    }

    /// Replaces the real TypeIDs in `text` from the response to `request` with fake ones,
    /// assigning new fake ids as needed.
    fn normalize(&mut self, text: &str, request: RequestHash) -> String {
        TYPE_ID
            .replace_all(text, |captures: &regex::Captures<'_>| {
                let real = &captures[0];
                if let Some(fake) = self.to_fake.get(real) {
                    return fake.0.clone();
                }
                let prefix = &captures[1];
                let count = self.counts.entry(request).or_default();
                // The suffix must start with `0`-`7` to be valid, and hex digits are within the
                // TypeID alphabet.
                let fake = format!("{prefix}_0{request}{count:09}");
                *count += 1;
                self.insert(RealId(real.to_owned()), FakeId(fake.clone()));
                fake
            })
            .into_owned()
    }

    /// Replaces the fake TypeIDs in `text` with the real ones they stand in for.
    ///
    /// Unknown ids, such as ones a case passes in its arguments, are passed through unchanged and
    /// mapped to themselves, so they stay the same if the server echoes them back. Such ids must
    /// not collide with the generated fakes or [`PROJECT_ID`], so don't copy them from recordings.
    fn denormalize(&mut self, text: &str) -> String {
        TYPE_ID
            .replace_all(text, |captures: &regex::Captures<'_>| {
                let fake = &captures[0];
                if let Some(real) = self.to_real.get(fake) {
                    return real.0.clone();
                }
                self.insert(RealId(fake.to_owned()), FakeId(fake.to_owned()));
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
    base_url: String,

    /// Stops the server when sent to, or dropped.
    shutdown: oneshot::Sender<()>,

    /// Runs the server, finishing once it has stopped and dropped its connections.
    thread: JoinHandle<Result<()>>,

    state: Arc<Mutex<State>>,

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

        let listener =
            std::net::TcpListener::bind("127.0.0.1:0").wrap_error("failed to start the proxy")?;
        listener
            .set_nonblocking(true)
            .wrap_error("failed to start the proxy")?;
        let port = listener
            .local_addr()
            .wrap_error("failed to start the proxy")?
            .port();
        let base_url = format!("http://127.0.0.1:{port}");

        let state = Arc::new(Mutex::new(State {
            backend,
            base_url: base_url.clone(),
            recorded: Vec::new(),
            errors: Vec::new(),
        }));

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .wrap_error("failed to start the proxy")?;
        let (shutdown, shutdown_received) = oneshot::channel();
        let thread = std::thread::spawn({
            let state = state.clone();
            move || {
                let result = runtime.block_on(serve(listener, state, shutdown_received));
                // Drops the connections that are still open, along with their references to the
                // state.
                drop(runtime);
                result
            }
        });

        Ok(Self {
            base_url,
            shutdown,
            thread,
            state,
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
        // The server only stops early on an error, which joining reports.
        let _ = self.shutdown.send(());
        self.thread
            .join()
            .map_err(|_| "the proxy thread panicked")?
            .wrap_error("the proxy failed")?;
        let state = Arc::into_inner(self.state)
            .ok_or("the proxy state is still in use after stopping")?
            .into_inner();

        if !state.errors.is_empty() {
            return Err(format!("proxy errors:\n{}", state.errors.join("\n")).into());
        }
        match state.backend {
            Backend::Replay(exchanges) => {
                if let Some(exchange) = exchanges.first() {
                    return Err(format!(
                        "{} recorded exchanges were not requested, starting with `{} {}`",
                        exchanges.len(),
                        exchange.request.method,
                        exchange.request.path,
                    )
                    .into());
                }
                Ok(())
            }
            Backend::Forward { record: true, .. } => {
                let mut recorded = state.recorded;
                sort(&mut recorded);
                save(&self.exchanges_path, &recorded)
            }
            Backend::Forward { record: false, .. } => Ok(()),
        }
    }
}

/// Accepts connections on `listener` and serves the requests on them until `shutdown` resolves.
///
/// Each connection is served concurrently, so connections a client keeps open can't hold up
/// others.
async fn serve(
    listener: std::net::TcpListener,
    state: Arc<Mutex<State>>,
    mut shutdown: oneshot::Receiver<()>,
) -> Result<()> {
    let listener =
        tokio::net::TcpListener::from_std(listener).wrap_error("failed to start the proxy")?;
    loop {
        let stream = tokio::select! {
            accepted = listener.accept() => {
                accepted.wrap_error("failed to accept a connection")?.0
            }
            _ = &mut shutdown => return Ok(()),
        };
        let state = state.clone();
        tokio::spawn(async move {
            let service = hyper::service::service_fn(move |request| handle(state.clone(), request));
            // The CLI hanging up early shows up in its own output.
            let _ = hyper::server::conn::http1::Builder::new()
                .serve_connection(TokioIo::new(stream), service)
                .await;
        });
    }
}

/// Answers `request` from `state`.
///
/// Requests are answered one at a time, in the order they finish arriving.
async fn handle(
    state: Arc<Mutex<State>>,
    request: hyper::Request<Incoming>,
) -> Result<hyper::Response<Full<Bytes>>, Infallible> {
    let (parts, body) = request.into_parts();
    let body = body
        .collect()
        .await
        .map(|body| body.to_bytes().to_vec())
        .wrap_error("failed to read the request body")
        .map_err(BoxError::from);
    let request = IncomingRequest {
        method: parts.method.as_str().to_owned(),
        path: parts
            .uri
            .path_and_query()
            .map_or("/", |path| path.as_str())
            .to_owned(),
        content_type: parts
            .headers
            .get(hyper::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
        body,
    };

    Ok(state.lock().await.handle(request).await)
}

/// A request as received by the proxy.
struct IncomingRequest {
    method: String,

    /// The path including any query string.
    path: String,

    content_type: Option<String>,

    /// The body, or why it couldn't be read.
    body: Result<Vec<u8>>,
}

struct State {
    backend: Backend,
    base_url: String,

    recorded: Vec<Exchange>,

    errors: Vec<String>,
}

impl State {
    async fn handle(&mut self, request: IncomingRequest) -> hyper::Response<Full<Bytes>> {
        self.exchange(request)
            .await
            .and_then(|response| self.http_response(&response))
            .unwrap_or_else(|error| {
                let message = Chain(&*error).to_string();
                self.errors.push(message.clone());
                let response = Response {
                    status: 500,
                    body: Some(Body::Text(message)),
                };
                self.http_response(&response)
                    .expect("an error response is valid")
            })
    }

    async fn exchange(&mut self, request: IncomingRequest) -> Result<Response> {
        let IncomingRequest {
            method,
            path,
            content_type,
            body,
        } = request;
        let body = body?;
        let json_body = match content_type.as_deref() {
            Some(content_type) if is_json(content_type) => Some(
                serde_json::from_slice(&body)
                    .wrap_error("failed to parse the JSON request body")?,
            ),
            _ => None,
        };
        let actual = Request {
            method,
            path,
            body: json_body,
        };

        match &mut self.backend {
            Backend::Replay(exchanges) => {
                let next = exchanges.first().ok_or_else(|| {
                    format!(
                        "unexpected request `{} {}` after all recorded exchanges",
                        actual.method, actual.path
                    )
                })?;
                let index = exchanges
                    .iter()
                    .position(|exchange| exchange.request == actual)
                    .ok_or_else(|| {
                        format!(
                            "request {actual:?} doesn't match any remaining recorded request, \
                             the next is {:?}",
                            next.request,
                        )
                    })?;
                Ok(exchanges.remove(index).response)
            }
            Backend::Forward { upstream, record } => {
                let response = upstream
                    .forward(&actual, content_type.as_deref(), body)
                    .await
                    .wrap_error(format_args!(
                        "failed to forward `{} {}`",
                        actual.method, actual.path
                    ))?;
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

    fn http_response(&self, response: &Response) -> Result<hyper::Response<Full<Bytes>>> {
        let (content_type, data) = match &response.body {
            None => (None, String::new()),
            Some(Body::Json(value)) => (Some("application/json"), value.to_string()),
            Some(Body::Text(text)) => (Some("text/plain"), text.clone()),
        };
        let data = data.replace(BASE_URL_PLACEHOLDER, &self.base_url);

        let mut builder = hyper::Response::builder().status(response.status);
        if let Some(content_type) = content_type {
            builder = builder.header(hyper::header::CONTENT_TYPE, content_type);
        }
        Ok(builder
            .body(Full::new(Bytes::from(data)))
            .wrap_error(format_args!("invalid response status {}", response.status))?)
    }
}
