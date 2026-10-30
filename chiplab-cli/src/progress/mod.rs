//! Progress reporting on stderr, in the style of cargo.
//!
//! Each line is an action, right-aligned so that all actions line up, followed by a message. Each
//! completed action leaves a line. When stderr is a terminal, the action in progress is shown below
//! them, with a spinner and optionally a bar.

mod test_terminal;
mod transfer;

use std::cell::RefCell;
use std::fmt;
use std::sync::LazyLock;
use std::time::Duration;

use console::Style;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use strum::VariantNames;

use self::test_terminal::TestTerminal;
pub use self::transfer::Transfer;
use crate::error::Result;

/// How often the spinner animates on a real terminal.
const TICK_INTERVAL: Duration = Duration::from_millis(100);

/// The spinner's characters, each different so that every tick is visible. The last is shown once
/// finished.
const TICK_CHARS: &str =
    "\u{280b}\u{2819}\u{2839}\u{2838}\u{283c}\u{2834}\u{2826}\u{2827}\u{2807}\u{280f} ";

/// The style of an action in progress.
static STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| in_progress_style(""));

/// The style of the line a completed action leaves when progress finishes.
static COMPLETED_STYLE: LazyLock<ProgressStyle> =
    LazyLock::new(|| ProgressStyle::with_template("{msg}").expect("the template is valid"));

/// The style of an action in progress, with `transfer` placed before its message.
fn in_progress_style(transfer: &str) -> ProgressStyle {
    let width = Action::MAX_WIDTH;
    let template = format!("{{spinner:.cyan}} {{prefix:>{width}.cyan.bold}} {transfer}{{msg}}");
    ProgressStyle::with_template(&template)
        .expect("the template is valid")
        .tick_chars(TICK_CHARS)
}

/// An action that a command reports taking.
#[derive(Debug, Clone, Copy, strum::IntoStaticStr, strum::VariantNames)]
pub enum Action {
    Uploading,
    Uploaded,
    Starting,
    Waiting,
    Finished,
    Failed,
    Downloading,
    Downloaded,
}

impl Action {
    /// The width that all actions are right-aligned to.
    const MAX_WIDTH: usize = {
        let mut max = 0;
        let mut index = 0;
        while index < Self::VARIANTS.len() {
            let width = Self::VARIANTS[index].len();
            if width > max {
                max = width;
            }
            index += 1;
        }
        max
    };

    fn as_str(self) -> &'static str {
        self.into()
    }
}

/// Reports the actions a command takes.
///
/// The action in progress is cleared when dropped.
#[derive(Debug)]
pub struct Progress {
    bar: ProgressBar,

    /// The line for the last completed action, not yet printed so that it's drawn together with
    /// whatever comes next.
    completed: RefCell<Option<String>>,
}

impl Progress {
    /// Creates progress reporting on stderr.
    pub fn new() -> Result<Self> {
        let bar = match TestTerminal::from_env()? {
            Some(terminal) => {
                // Colored as on a real terminal. Styles in the template follow stdout's setting
                // when drawing to a custom terminal, so that needs enabling too.
                console::set_colors_enabled_stderr(true);
                console::set_colors_enabled(true);
                ProgressBar::with_draw_target(
                    None,
                    ProgressDrawTarget::term_like(Box::new(terminal)),
                )
                .with_style(STYLE.clone())
            }
            None => {
                let bar = ProgressBar::new_spinner().with_style(STYLE.clone());
                if !bar.is_hidden() {
                    bar.enable_steady_tick(TICK_INTERVAL);
                }
                bar
            }
        };

        Ok(Self {
            bar,
            completed: RefCell::default(),
        })
    }

    /// Shows `action` in progress on `message`, until the next action starts or progress finishes.
    ///
    /// Only visible on a terminal.
    pub fn start(&self, action: Action, message: impl Into<String>) {
        self.bar.set_style(STYLE.clone());
        self.show(action, message.into());
    }

    /// Like [`Progress::start`], but also reported when stderr isn't a terminal.
    pub fn start_or_print(&self, action: Action, message: impl Into<String>) {
        let message = message.into();
        if self.bar.is_hidden() {
            self.report(Style::new().cyan().bold(), action, message);
        } else {
            self.start(action, message);
        }
    }

    /// Like [`Progress::start`], with a bar for a transfer of `total` steps.
    pub fn start_transfer(
        &self,
        action: Action,
        message: impl Into<String>,
        total: u64,
    ) -> Transfer<'_> {
        // Neither changing the style nor the position draws, so they're drawn along with the
        // action.
        self.bar.set_style(transfer::style(total));
        let _ = self.bar.clone().with_position(0);
        self.show(action, message.into());

        Transfer::new(self)
    }

    /// Reports that `action` completed on `message`.
    pub fn complete(&self, action: Action, message: impl fmt::Display) {
        self.report(Style::new().green().bold(), action, message);
    }

    /// Reports that `action` completed on `message`, but with problems.
    pub fn warn(&self, action: Action, message: impl fmt::Display) {
        self.report(Style::new().yellow().bold(), action, message);
    }

    /// Reports that `action` failed on `message`.
    pub fn fail(&self, action: Action, message: impl fmt::Display) {
        self.report(Style::new().red().bold(), action, message);
    }

    /// Advances the spinner to show that the action in progress is still progressing.
    ///
    /// Also draws any change to the action in progress, along with the last completed action if
    /// not yet drawn.
    pub fn tick(&self) {
        match self.completed.take() {
            Some(completed) => self.bar.println(completed),
            // A real terminal is redrawn regularly anyway, in which case this does nothing.
            // Otherwise this draws immediately, so that test frames don't depend on timing.
            None => self.bar.tick(),
        }
    }

    /// Prints `line` to stderr, above the action in progress when it's visible.
    pub fn println(&self, line: impl AsRef<str>) {
        let line = line.as_ref();
        if self.bar.is_hidden() {
            eprintln!("{line}");
            return;
        }

        match self.completed.take() {
            Some(completed) => self.bar.println(format!("{completed}\n{line}")),
            None => self.bar.println(line),
        }
    }

    /// Removes the action in progress, so that later output to stderr isn't mixed up with it.
    pub fn finish(&self) {
        match self.completed.take() {
            Some(completed) => {
                // Changing the style doesn't draw, so the action in progress is replaced by the
                // completed line in one go.
                self.bar.set_style(COMPLETED_STYLE.clone());
                self.bar.finish_with_message(completed);
            }
            None => self.bar.finish_and_clear(),
        }
    }

    fn show(&self, action: Action, message: String) {
        // Doesn't draw, unlike `set_prefix` and `set_message`, so that both are drawn at once.
        let _ = self
            .bar
            .clone()
            .with_prefix(action.as_str())
            .with_message(message);
        self.tick();
    }

    fn report(&self, style: Style, action: Action, message: impl fmt::Display) {
        // Leaves room for the spinner, so that the action lines up with the one in progress.
        let indent = " ".repeat(2 + Action::MAX_WIDTH + 1);
        // Any further lines of the message line up under its first.
        let message = message.to_string().replace('\n', &format!("\n{indent}"));
        let line = format!(
            "  {:>width$} {message}",
            style.for_stderr().apply_to(action.as_str()),
            width = Action::MAX_WIDTH,
        );

        if self.bar.is_hidden() {
            eprintln!("{line}");
            return;
        }

        if let Some(previous) = self.completed.replace(Some(line)) {
            self.bar.println(previous);
        }
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        if !self.bar.is_finished() {
            self.finish();
        }
    }
}
