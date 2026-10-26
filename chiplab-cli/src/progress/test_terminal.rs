use std::io::Write;

use indicatif::TermLike;

use crate::error::{Result, ResultExt};

/// Forces progress to be drawn to a [`TestTerminal`] of the given size, `<COLUMNS>×<ROWS>`.
const TEST_TERMINAL: &str = "CHIPLAB_TEST_TERMINAL";

/// Marks the end of a frame drawn to a [`TestTerminal`].
///
/// An OSC sequence, so terminals ignore it.
const FRAME_END: &str = "\x1b]chiplab-frame-end\x07";

/// A fixed-size terminal on stderr, for tests.
///
/// Every update is drawn immediately and followed by [`FRAME_END`], so that each frame can be
/// snapshotted deterministically.
#[derive(Debug)]
pub(super) struct TestTerminal {
    columns: u16,
    rows: u16,
}

impl TestTerminal {
    /// Returns the terminal requested by [`TEST_TERMINAL`], if any.
    pub(super) fn from_env() -> Result<Option<Self>> {
        let size = match std::env::var(TEST_TERMINAL) {
            Ok(size) => size,
            Err(std::env::VarError::NotPresent) => return Ok(None),
            Err(error) => {
                Err(error).wrap_error(format_args!("failed to read `{TEST_TERMINAL}`"))?
            }
        };

        let terminal =
            Self::parse(&size).wrap_error(format_args!("failed to parse `{TEST_TERMINAL}`"))?;

        Ok(Some(terminal))
    }

    /// Parses a terminal size of the form `<COLUMNS>×<ROWS>`.
    fn parse(size: &str) -> Result<Self> {
        let (columns, rows) = size
            .split_once('\u{00D7}')
            .ok_or_else(|| format!("expected `<COLUMNS>×<ROWS>`, got `{size}`"))?;

        Ok(Self {
            columns: columns.parse().wrap_error("invalid number of columns")?,
            rows: rows.parse().wrap_error("invalid number of rows")?,
        })
    }

    fn write(&self, data: impl std::fmt::Display) -> std::io::Result<()> {
        write!(std::io::stderr(), "{data}")
    }

    fn move_cursor(&self, n: usize, direction: char) -> std::io::Result<()> {
        if n == 0 {
            return Ok(());
        }

        self.write(format_args!("\x1b[{n}{direction}"))
    }
}

impl TermLike for TestTerminal {
    fn width(&self) -> u16 {
        self.columns
    }

    fn height(&self) -> u16 {
        self.rows
    }

    fn move_cursor_up(&self, n: usize) -> std::io::Result<()> {
        self.move_cursor(n, 'A')
    }

    fn move_cursor_down(&self, n: usize) -> std::io::Result<()> {
        self.move_cursor(n, 'B')
    }

    fn move_cursor_right(&self, n: usize) -> std::io::Result<()> {
        self.move_cursor(n, 'C')
    }

    fn move_cursor_left(&self, n: usize) -> std::io::Result<()> {
        self.move_cursor(n, 'D')
    }

    fn write_line(&self, line: &str) -> std::io::Result<()> {
        self.write(format_args!("{line}\n"))
    }

    fn write_str(&self, string: &str) -> std::io::Result<()> {
        self.write(string)
    }

    fn clear_line(&self) -> std::io::Result<()> {
        self.write("\r\x1b[2K")
    }

    fn flush(&self) -> std::io::Result<()> {
        self.write(FRAME_END)?;
        std::io::stderr().flush()
    }
}
