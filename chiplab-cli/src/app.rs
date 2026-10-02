use anyhow::Context;
use chiplab_api::apis::configuration::Configuration;

const DEFAULT_URL: &str = "https://chiplab.veecle.ai";

/// State shared by all subcommands.
#[derive(Debug)]
pub struct App {
    /// The connection to the Chiplab API.
    pub api: Configuration,
}

impl App {
    /// Creates the app state from the environment.
    ///
    /// - `CHIPLAB_URL`: the Chiplab instance to connect to, defaults to `https://chiplab.veecle.ai`.
    /// - `CHIPLAB_TOKEN`: the personal access token to authenticate with (`veecle_pat_…`).
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            api: api_configuration()?,
        })
    }
}

fn api_configuration() -> anyhow::Result<Configuration> {
    let url = match std::env::var("CHIPLAB_URL") {
        Ok(url) => url,
        Err(std::env::VarError::NotPresent) => DEFAULT_URL.to_owned(),
        Err(error) => return Err(error).context("failed to read `CHIPLAB_URL`"),
    };
    let token = std::env::var("CHIPLAB_TOKEN").context("failed to read `CHIPLAB_TOKEN`")?;

    let user_agent = concat!(
        env!("CARGO_PKG_NAME"),
        "/",
        env!("CARGO_PKG_VERSION"),
        " ",
        "(",
        env!("CARGO_PKG_REPOSITORY"),
        ")",
    );
    let client = reqwest::Client::builder()
        .user_agent(user_agent)
        .build()
        .context("failed to create the HTTP client")?;

    Ok(Configuration {
        base_path: url.trim_end_matches('/').to_owned(),
        client,
        bearer_access_token: Some(token),
        user_agent: None, // handled by the client
        basic_auth: None,
        oauth_access_token: None,
        api_key: None,
    })
}
