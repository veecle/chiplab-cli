/// State shared by all subcommands.
#[derive(Debug)]
pub struct App {}

impl App {
    /// Creates the app state from the environment.
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {})
    }
}
