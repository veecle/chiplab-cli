//! Renders a [`Run`] whose stderr is a terminal as an SVG: the run as text, followed by each frame.

use crate::run::Run;

/// The space between the terminals, in pixels.
const GAP: usize = 20;

/// The height of a screen's label, in pixels.
const LABEL_HEIGHT: usize = 24;

/// The offset of a label's baseline from its top, in pixels.
const LABEL_BASELINE: usize = 16;

pub fn render(run: &Run) -> String {
    let without_frames = Run {
        frames: Vec::new(),
        ..run.clone()
    };

    let mut layout = Layout::default();
    let first = layout.terminal(anstyle_svg::Term::new(), trim(&without_frames.to_string()));

    // The run is the widest terminal, so the frames are matched to its width.
    let padding = Terminal::new(anstyle_svg::Term::new().min_width_px(0).render_svg("")).width;
    let term = anstyle_svg::Term::new().min_width_px(first.width - padding);
    for (number, frame) in std::iter::zip(1.., &run.frames) {
        layout.label(&format!("stderr frame {number}"));
        layout.terminal(term, trim(frame));
    }

    let Layout {
        elements,
        width,
        height,
    } = layout;
    let elements = elements.join("\n");
    let width = width + 2 * GAP;
    let height = height + 2 * GAP;
    format!(
        r##"<svg width="{width}px" height="{height}px" style="background: #1E1E1E" xmlns="http://www.w3.org/2000/svg">
<style>
  .label {{ fill: #DDDDDD; font: bold 14px SFMono-Regular, Consolas, Liberation Mono, Menlo, monospace; }}
</style>
<g transform="translate({GAP} {GAP})">
{elements}
</g>
</svg>
"##
    )
}

/// Elements placed one below the other.
#[derive(Default)]
struct Layout {
    elements: Vec<String>,
    width: usize,
    height: usize,
}

impl Layout {
    /// Places a label for what follows, after a gap.
    fn label(&mut self, text: &str) {
        self.height += GAP;
        self.elements.push(format!(
            r#"<text x="0px" y="{}px" class="label">{text}</text>"#,
            self.height + LABEL_BASELINE,
        ));
        self.height += LABEL_HEIGHT;
    }

    /// Places a terminal rendered by `term` showing `text`.
    fn terminal(&mut self, term: anstyle_svg::Term, text: &str) -> Terminal {
        let terminal = Terminal::new(term.render_svg(text));
        self.elements.push(terminal.place(0, self.height));
        self.width = self.width.max(terminal.width);
        self.height += terminal.height;
        terminal
    }
}

/// Removes the final newline, which would otherwise be rendered as an extra blank line.
fn trim(text: &str) -> &str {
    text.strip_suffix('\n').unwrap_or(text)
}

/// A terminal rendered as an SVG.
struct Terminal {
    svg: String,
    width: usize,
    height: usize,
}

impl Terminal {
    fn new(svg: String) -> Self {
        let width = dimension(&svg, "width");
        let height = dimension(&svg, "height");
        Self { svg, width, height }
    }

    /// The terminal as an element placed at `x` and `y`.
    fn place(&self, x: usize, y: usize) -> String {
        let svg = self
            .svg
            .strip_prefix("<svg ")
            .expect("rendered terminals are SVGs");
        format!(r#"<svg x="{x}px" y="{y}px" {}"#, svg.trim_end())
    }
}

/// The value of the `name` dimension of the SVG's root element, in pixels.
fn dimension(svg: &str, name: &str) -> usize {
    svg.split_once(&format!(r#" {name}=""#))
        .and_then(|(_, rest)| rest.split_once(r#"px""#))
        .and_then(|(value, _)| value.parse().ok())
        .unwrap_or_else(|| panic!("rendered terminals have a `{name}` in pixels"))
}
