//! Emulates the terminal the binary's stderr is shown on when it believes it's a terminal.

use crate::push_trimmed;

/// The size of the terminal.
///
/// Wide enough that lines don't wrap, as wrapped lines make diffs of the SVGs harder to read, and
/// tall enough that nothing scrolls off, as the parser keeps no scrollback.
const COLUMNS: u16 = 240;
const ROWS: u16 = 200;

/// The variable telling the binary to draw to a test terminal, must match the binary's.
pub const ENV: &str = "CHIPLAB_TEST_TERMINAL";

/// Marks the end of each frame the binary draws to a test terminal, must match the binary's.
const FRAME_END: &str = "\x1b]chiplab-frame-end\x07";

/// The value telling the binary to draw to a test terminal of this size.
pub fn env_value() -> String {
    format!("{COLUMNS}\u{00D7}{ROWS}")
}

/// The screens of a terminal that `output` is written to: one after each frame drawn, skipping
/// repeats, and the final one.
pub fn screens(output: &[u8]) -> (Vec<String>, String) {
    let rows = std::num::NonZeroU16::new(ROWS).expect("the terminal has rows");
    let columns = std::num::NonZeroU16::new(COLUMNS).expect("the terminal has columns");
    let mut parser = vt100::Parser::new(rows, columns, 0);

    let output = String::from_utf8_lossy(output);
    let mut chunks = output.split(FRAME_END).peekable();
    let mut frames = Vec::new();
    let mut previous = None;
    loop {
        let chunk = chunks
            .next()
            .expect("splitting always yields at least one chunk");
        // A terminal's line discipline usually translates newlines for output, the parser doesn't.
        parser.process(chunk.replace('\n', "\r\n").as_bytes());
        // Row by row, as indicatif pads its lines to the full width to end them, and `contents`
        // joins a full row onto the next one as if it were wrapped.
        let screen = (0..ROWS)
            .map(|row| render_row(parser.screen(), row))
            .collect::<Vec<_>>()
            .join("\n");

        let mut contents = String::new();
        push_trimmed(&mut contents, screen.trim_end().as_bytes());

        if chunks.peek().is_none() {
            return (frames, contents);
        }
        if previous.as_ref() != Some(&screen) {
            frames.push(contents);
            previous = Some(screen);
        }
    }
}

/// Renders `row` of `screen` with its colors and attributes as SGR escape sequences, without
/// trailing blank cells.
fn render_row(screen: &vt100::Screen, row: u16) -> String {
    let cells = (0..COLUMNS)
        .filter_map(|column| screen.cell(row, column))
        .filter(|cell| !cell.is_wide_continuation())
        .map(|cell| {
            let contents = if cell.has_contents() {
                cell.contents()
            } else {
                " "
            };
            (contents, sgr(cell))
        })
        .collect::<Vec<_>>();
    let end = cells
        .iter()
        .rposition(|(contents, sgr)| *contents != " " || !sgr.is_empty())
        .map_or(0, |last| last + 1);

    let mut rendered = String::new();
    let mut current = "";
    for (contents, sgr) in &cells[..end] {
        if sgr != current {
            if !current.is_empty() {
                rendered.push_str("\x1b[0m");
            }
            rendered.push_str(sgr);
            current = sgr;
        }
        rendered.push_str(contents);
    }
    if !current.is_empty() {
        rendered.push_str("\x1b[0m");
    }

    rendered
}

/// The SGR escape sequence selecting the colors and attributes of `cell`, empty for the defaults.
fn sgr(cell: &vt100::Cell) -> String {
    let mut parameters = Vec::new();
    for (enabled, parameter) in [
        (cell.bold(), "1"),
        (cell.dim(), "2"),
        (cell.italic(), "3"),
        (cell.underline(), "4"),
        (cell.inverse(), "7"),
    ] {
        if enabled {
            parameters.push(parameter.to_owned());
        }
    }
    for (color, base, bright_base, extended) in
        [(cell.fgcolor(), 30, 90, 38), (cell.bgcolor(), 40, 100, 48)]
    {
        match color {
            vt100::Color::Default => {}
            vt100::Color::Idx(index @ 0..8) => parameters.push((base + index).to_string()),
            vt100::Color::Idx(index @ 8..16) => {
                parameters.push((bright_base + index - 8).to_string());
            }
            vt100::Color::Idx(index) => parameters.push(format!("{extended};5;{index}")),
            vt100::Color::Rgb(red, green, blue) => {
                parameters.push(format!("{extended};2;{red};{green};{blue}"));
            }
        }
    }

    if parameters.is_empty() {
        String::new()
    } else {
        format!("\x1b[{}m", parameters.join(";"))
    }
}
