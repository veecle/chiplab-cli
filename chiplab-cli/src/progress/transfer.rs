use std::fmt;
use std::sync::LazyLock;

use indicatif::{ProgressState, ProgressStyle};

use super::{Action, Progress, in_progress_style};

/// How many characters wide a transfer's bar is.
const BAR_WIDTH: u64 = 20;

/// The style of an action in progress making a transfer, missing the `transfer` key.
static STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| in_progress_style("{transfer} "));

/// The style of an action in progress making a transfer of `total` steps.
pub(super) fn style(total: u64) -> ProgressStyle {
    STYLE.clone().with_key(
        "transfer",
        move |state: &ProgressState, w: &mut dyn fmt::Write| {
            let done = state.pos();
            let filled = (done.min(total) * BAR_WIDTH)
                .checked_div(total)
                .unwrap_or(BAR_WIDTH);
            let filled = usize::try_from(filled).expect("the bar fits in memory");
            let width = usize::try_from(BAR_WIDTH).expect("the bar fits in memory");

            let _ = write!(
                w,
                "[{:=<filled$}{:empty$}] {done}/{total}",
                "",
                "",
                empty = width - filled
            );
        },
    )
}

/// Tracks the steps of a transfer started with [`Progress::start_transfer`].
///
/// [`Progress::start_transfer`]: super::Progress::start_transfer
#[derive(Debug)]
pub struct Transfer<'a> {
    progress: &'a Progress,
}

impl<'a> Transfer<'a> {
    pub(super) fn new(progress: &'a Progress) -> Self {
        Self { progress }
    }

    /// Reports that `action` completed on `message`, as one more step of the transfer.
    ///
    /// When given, `next` replaces the message of the transfer in progress.
    pub fn complete(&self, action: Action, message: impl fmt::Display, next: Option<String>) {
        self.progress.complete(action, message);
        self.step(next);
        self.progress.tick();
    }

    /// Reports that `action` failed on `message`, as one more step of the transfer.
    pub fn fail(&self, action: Action, message: impl fmt::Display) {
        self.progress.fail(action, message);
        self.step(None);
        self.progress.tick();
    }

    /// Advances the transfer by one step, replacing its message with `next` when given.
    ///
    /// Doesn't draw, unlike `inc` and `set_message`, so that it's drawn along with the step.
    fn step(&self, next: Option<String>) {
        let bar = &self.progress.bar;
        let bar = bar.clone().with_position(bar.position() + 1);
        if let Some(next) = next {
            let _ = bar.with_message(next);
        }
    }
}
