//! Error handling, adapted from `vip-error`.
//!
//! Errors are boxed trait objects, wrapped with ad-hoc messages via [`WrapError`] and tagged with
//! an exit status via [`WithExitCode`]. [`report`] prints the error chain and picks the status.

use std::error::Error;
use std::fmt::{self, Display, Formatter, Write};
use std::process::ExitCode;

pub type BoxError = Box<dyn Error + Send + Sync>;

pub type Result<T, E = BoxError> = std::result::Result<T, E>;

/// Exit status for errors without an [`ExitCodeError`] in their chain.
const DEFAULT_EXIT_CODE: u8 = 1;

/// Wraps an error in a [`WrappedError`] carrying `message`, keeping the original as its source.
pub trait WrapError<T> {
    /// Wraps the error with `message`.
    ///
    /// `message` is only formatted if there is an error to wrap, so passing [`format_args!`] is
    /// cheap on the success path.
    fn wrap_error(self, message: impl Display) -> Result<T, WrappedError>;

    /// Like [`WrapError::wrap_error`], but also defers producing the message until there is an
    /// error to wrap.
    fn wrap_error_with<D: Display>(self, message: impl FnOnce() -> D) -> Result<T, WrappedError>;
}

/// An ad-hoc message layered on top of a source error, see [`WrapError`].
#[derive(Debug)]
pub struct WrappedError {
    message: String,
    source: BoxError,
}

impl Display for WrappedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for WrappedError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&*self.source)
    }
}

impl<T, E> WrapError<T> for Result<T, E>
where
    E: Into<BoxError>,
{
    fn wrap_error(self, message: impl Display) -> Result<T, WrappedError> {
        self.wrap_error_with(|| message)
    }

    fn wrap_error_with<D: Display>(self, message: impl FnOnce() -> D) -> Result<T, WrappedError> {
        self.map_err(|error| WrappedError {
            message: message().to_string(),
            source: error.into(),
        })
    }
}

/// Tags an error with the exit status to use if it ends the process, see [`ExitCodeError`].
pub trait WithExitCode<T> {
    fn with_exit_code(self, code: u8) -> Result<T, ExitCodeError>;
}

impl<T, E> WithExitCode<T> for Result<T, E>
where
    E: Into<BoxError>,
{
    fn with_exit_code(self, code: u8) -> Result<T, ExitCodeError> {
        self.map_err(|error| ExitCodeError::new(code, error))
    }
}

/// An error tagged with an exit status.
///
/// Transparent: displays as the wrapped error and forwards its source, so it adds no line to the
/// reported chain. If the chain holds several, [`report`] uses the outermost.
#[derive(Debug)]
pub struct ExitCodeError {
    code: u8,
    error: BoxError,
}

impl ExitCodeError {
    pub fn new(code: u8, error: impl Into<BoxError>) -> Self {
        Self {
            code,
            error: error.into(),
        }
    }
}

impl Display for ExitCodeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.error, f)
    }
}

impl Error for ExitCodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.error.source()
    }
}

/// Exits with the given status without printing anything, for outcomes already reported.
#[derive(Debug)]
pub struct Silent(pub u8);

impl Display for Silent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "exiting with status {}", self.0)
    }
}

impl Error for Silent {}

/// Prints `error` with its chain to stderr, unless it is [`Silent`], and returns its exit status.
pub fn report(error: &BoxError) -> ExitCode {
    let error: &(dyn Error + 'static) = &**error;

    if !error.is::<Silent>() {
        eprintln!("error: {}", Chain(error));
    }

    ExitCode::from(exit_code(error))
}

/// The status of the outermost [`ExitCodeError`] or [`Silent`] in the chain.
fn exit_code(error: &(dyn Error + 'static)) -> u8 {
    let mut next = Some(error);

    while let Some(error) = next {
        if let Some(error) = error.downcast_ref::<ExitCodeError>() {
            return error.code;
        }
        if let Some(Silent(code)) = error.downcast_ref::<Silent>() {
            return *code;
        }
        next = error.source();
    }

    DEFAULT_EXIT_CODE
}

/// Formats an error followed by its full `source()` chain.
struct Chain<'a>(&'a (dyn Error + 'static));

impl Display for Chain<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)?;

        let mut source = self.0.source();

        if source.is_some() {
            write!(f, "\nCaused by:")?;

            while let Some(cause) = source {
                f.write_str("\n  - ")?;
                write!(Indented::new(f), "{cause}")?;
                source = cause.source();
            }
        }

        Ok(())
    }
}

/// Indents every line after the first to align under a `  - ` bullet.
struct Indented<'a, 'b> {
    f: &'a mut Formatter<'b>,
    at_line_start: bool,
}

impl<'a, 'b> Indented<'a, 'b> {
    fn new(f: &'a mut Formatter<'b>) -> Self {
        Self {
            f,
            at_line_start: false,
        }
    }
}

impl Write for Indented<'_, '_> {
    fn write_str(&mut self, chunk: &str) -> fmt::Result {
        for line in chunk.split_inclusive('\n') {
            if self.at_line_start {
                self.f.write_str("    ")?;
            }
            self.f.write_str(line)?;
            self.at_line_start = line.ends_with('\n');
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fmt::{self, Display, Formatter};

    use super::{BoxError, Chain, Silent, WithExitCode, WrapError, exit_code};

    /// Writes the newline and the following line as separate chunks.
    #[derive(Debug)]
    struct Chunked;

    impl Display for Chunked {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            f.write_str("first")?;
            f.write_str("\n")?;
            f.write_str("second\nthird")
        }
    }

    impl Error for Chunked {}

    fn boxed<E: Into<BoxError>>(result: Result<(), E>) -> BoxError {
        result.unwrap_err().into()
    }

    #[test]
    fn indents_multi_line_causes() {
        let error = boxed(Err::<(), _>(Chunked).wrap_error("outer"));
        assert_eq!(
            Chain(&*error).to_string(),
            "outer\nCaused by:\n  - first\n    second\n    third",
        );
    }

    #[test]
    fn exit_code_is_transparent_in_chain() {
        let error = boxed(
            Err::<(), _>("inner")
                .with_exit_code(4)
                .wrap_error("middle")
                .wrap_error("outer"),
        );
        assert_eq!(
            Chain(&*error).to_string(),
            "outer\nCaused by:\n  - middle\n  - inner",
        );
    }

    #[test]
    fn exit_code_found_under_wrapped_errors() {
        let error = boxed(
            Err::<(), _>("inner")
                .with_exit_code(4)
                .wrap_error("middle")
                .wrap_error("outer"),
        );
        assert_eq!(exit_code(&*error), 4);
    }

    #[test]
    fn outermost_exit_code_wins() {
        let error = boxed(
            Err::<(), _>("inner")
                .with_exit_code(4)
                .wrap_error("middle")
                .with_exit_code(3),
        );
        assert_eq!(exit_code(&*error), 3);
    }

    #[test]
    fn exit_code_defaults_to_1() {
        let error = boxed(Err::<(), _>("inner").wrap_error("outer"));
        assert_eq!(exit_code(&*error), 1);
    }

    #[test]
    fn silent_exit_code() {
        let error = boxed(Err::<(), _>(Silent(4)));
        assert_eq!(exit_code(&*error), 4);
    }
}
