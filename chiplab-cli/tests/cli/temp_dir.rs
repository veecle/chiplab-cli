//! Redaction of temporary directories in text.
//!
//! The redaction is one-way, so it only suits paths the CLI doesn't send back to Chiplab.

use std::sync::LazyLock;

use regex::Regex;

/// Stands in for temporary directories in redacted text.
pub const PLACEHOLDER: &str = "[CHIPLAB_TEMP_DIR]";

/// Matches a path to a temporary directory under a `tmp` directory, e.g. `./tmp/foo-QHpLw0`.
///
/// The temporary directory is one created by `mktemp` from a `<prefix>-XXXXXX` template, or by the
/// `tempfile` crate (`.tmpXXXXXX`). Path segments may only contain ASCII alphanumerics, `_`, `.`
/// and `-`; any other character ends the path.
static PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:[A-Za-z0-9_.-]*/)*\btmp/(?:[A-Za-z0-9_.-]+-|\.tmp)[A-Za-z0-9]{6}\b")
        .expect("the temporary directory pattern is valid")
});

/// Replaces the paths to temporary directories in `text` with [`PLACEHOLDER`], keeping any path
/// within them.
pub fn redact(text: &str) -> String {
    PATTERN.replace_all(text, PLACEHOLDER).into_owned()
}
