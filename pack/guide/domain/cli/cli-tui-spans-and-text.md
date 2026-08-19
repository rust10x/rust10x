# CLI TUI Spans & Text Formatting

## Purpose

This guide details span-level construction, text formatting conventions, marker layout schemes, path segmentation, and style management used across CLI views and components.

## Direct Span-Level Control

Constructing and managing individual `Span<'static>` instances directly, rather than relying on high-level opaque widgets, provides essential control points:

- Responsive width measurement: Span widths can be computed prior to buffer rendering using `UiExt::x_width`, allowing dynamic truncation, padding, and alignment adjustments based on container bounds.
- Selective styling: Individual token segments (markers, labels, values, file paths, status icons) maintain independent colors and modifiers without string splitting at render time.
- Dynamic hover and interaction states: Mutable span slices (`&mut [Span]`) allow two-pass hover highlighting to update foreground colors, backgrounds, or underline modifiers immediately before passing lines to Ratatui widgets.
- Precise interaction mapping: `LinkZone` hit testing relies on exact span byte offsets and calculated visual widths to resolve clicks down to specific path tokens or broad grouped blocks.

## Owned Render Output

Content builders generally return owned values:

- `Vec<Line<'static>>` for multiline content.
- `Vec<Span<'static>>` for a single row or reusable span group.
- `Line::from(spans)` for assembling a line from styled spans.
- `Span::raw`, `Span::styled`, and style extensions for individual content segments.

Owned output is important because views may mutate styles after content construction. Hover handling changes the foreground, background, or complete style of spans before the lines are rendered.

## Span Extension Helper (`UiExt`)

Mutable span slices and lines implement `UiExt` to enable fluent style mutations during hover passes and width calculations:

```rust
use ratatui::style::Color;
use ratatui::text::{Line, Span};

pub trait UiExt {
	fn x_bg(self, color: Color) -> Self;
	fn x_fg(self, color: Color) -> Self;
	fn x_width(&self) -> u16;
}

impl<'a> UiExt for &'a mut [Span<'static>] {
	fn x_bg(self, color: Color) -> Self {
		for span in self.iter_mut() {
			span.style.bg = color.into();
		}
		self
	}

	fn x_fg(self, color: Color) -> Self {
		for span in self.iter_mut() {
			span.style.fg = color.into();
		}
		self
	}

	fn x_width(&self) -> u16 {
		self.iter().map(|span| span.width() as u16).sum()
	}
}
```

## Text Formatting Patterns

Consistent text formatting ensures visual stability and predictable alignment across varied terminal dimensions:

- Number and index padding: Use fixed-width padding based on total collection length (for example `text::num_pad_for_len(idx, total_len)`) so list item numbers align vertically.
- Truncation with ellipsis: For single-line constraints, truncate long descriptions with trailing ellipsis (for example `truncate_with_ellipsis(text, max_width, "..")` or `truncate(text, max_width)`) to prevent uncontrolled wrapping.
- Tab normalization: Tab characters (`\t`) must be converted to uniform spaces (typically 4 spaces) before width calculations and text wrapping to prevent coordinate drift.
- Alignment formatting: Format string macros (such as `format!("{label:<width$}")` for left alignment or right-aligned Paragraphs) guarantee that bounding boxes remain exact.
- Numeric metrics: Duration and cost formatting helper functions produce compact, human-readable strings with fixed or predictable bounds.

## Marker Section Layout

`ui_for_marker_section_str` produces marker-prefixed, wrapped, optionally path-aware content and can register link zones while it builds the lines:

- A marker is right-aligned to a minimum width (`MARKER_MIN_WIDTH`, default 10).
- A one-character spacer follows the marker.
- Optional prefix spans follow the spacer.
- Content is wrapped to the remaining width.
- Continuation lines receive blank marker indentation.
- Path segments receive path styling and optional `OpenFile` actions.
- Content segments may receive a grouped action such as `ToClipboardCopy`.
- The line accumulator advances the current link-zone line after the section.

```rust
use std::borrow::Cow;
use ratatui::style::Style;
use ratatui::text::{Line, Span};

const MARKER_MIN_WIDTH: usize = 10;

pub fn new_marker(marker_txt: &str, marker_style: Style) -> Span<'static> {
	let marker_width = marker_txt.chars().count().max(MARKER_MIN_WIDTH);
	Span::styled(format!("{marker_txt:>marker_width$}"), marker_style)
}

pub fn ui_for_marker_section_str(
	content: &str,
	(marker_txt, marker_style): (&str, Style),
	max_width: u16,
	content_prefix: Option<&Vec<Span<'static>>>,
	mut link_zones: Option<&mut LinkZones>,
	action: Option<UiAction>,
	path_color: Option<Color>,
) -> Vec<Line<'static>> {
	let spacer = " ";
	let width_spacer = spacer.len();
	let marker_width = marker_txt.chars().count().max(MARKER_MIN_WIDTH);
	let width_content = (max_width as usize).saturating_sub(marker_width + width_spacer);

	let mark_span = new_marker(marker_txt, marker_style);
	let spaced_content: Cow<str> = if content.contains('\t') {
		Cow::Owned(content.replace('\t', "    "))
	} else {
		Cow::Borrowed(content)
	};

	let msg_wrap = textwrap::wrap(&spaced_content, width_content);
	let msg_wrap_len = msg_wrap.len();
	let mut msg_wrap_iter = msg_wrap.into_iter();

	let group_id = if let Some(lz) = link_zones.as_mut() && action.is_some() {
		Some(lz.start_group())
	} else {
		None
	};

	let mut lines = Vec::new();

	let mut push_line_fn = |rel_line_idx: usize,
	                        prefix_spans: Vec<Span<'static>>,
	                        line_content: &str,
	                        mut lz_opt: Option<&mut LinkZones>,
	                        gid_opt: Option<u32>,
	                        main_action_opt: Option<&UiAction>| {
		let mut spans = prefix_spans;
		let content_span_start = spans.len();
		let segments = segment_line_path(line_content);

		for seg in segments {
			let style = if seg.file_path.is_some() {
				style_text_path(false, path_color)
			} else {
				style::STL_SECTION_TXT
			};
			let span_idx = spans.len();
			spans.push(Span::styled(seg.text, style));

			if let Some(lz) = lz_opt.as_mut() {
				if let Some(path) = seg.file_path {
					lz.push_link_zone(rel_line_idx, span_idx, 1, UiAction::OpenFile(path.to_string()));
				} else if let (Some(gid), Some(act)) = (gid_opt, main_action_opt) {
					lz.push_group_zone(rel_line_idx, span_idx, 1, gid, act.clone());
				}
			}
		}

		if let (Some(lz), Some(gid), Some(act)) = (lz_opt, gid_opt, main_action_opt) {
			lz.push_group_zone(
				rel_line_idx,
				content_span_start,
				spans.len() - content_span_start,
				gid,
				act.clone(),
			);
		}

		lines.push(Line::from(spans));
	};

	let first_content = msg_wrap_iter.next().unwrap_or_default();
	let mut first_prefix = vec![mark_span, Span::raw(spacer)];
	if let Some(spans_prefix) = content_prefix {
		first_prefix.extend(spans_prefix.to_vec());
	}
	push_line_fn(0, first_prefix, &first_content, link_zones.as_deref_mut(), group_id, action.as_ref());

	if msg_wrap_len > 1 {
		let left_spacing = " ".repeat(marker_width + width_spacer);
		for (i, line_content) in msg_wrap_iter.enumerate() {
			let mut other_prefix = vec![Span::raw(left_spacing.clone())];
			if let Some(spans_prefix) = content_prefix {
				other_prefix.extend(spans_prefix.to_vec());
			}
			push_line_fn(i + 1, other_prefix, &line_content, link_zones.as_deref_mut(), group_id, action.as_ref());
		}
	}

	if let Some(lz) = link_zones.as_mut() {
		lz.inc_current_line_by(lines.len());
	}

	lines
}
```

## Wrapping and Width Control

Content width is derived from the available width after subtracting the marker, spacer, and optional prefix widths.

Important width rules:

- The caller must provide a width that includes enough room for the marker and spacer.
- Scrollbar space is reserved by passing a reduced width such as `area.width - 3`.
- Narrow terminal layouts need explicit protection before subtracting fixed widths.
- Builders should prefer `saturating_sub` when a width can be zero or smaller than the expected component width.
- Task facade methods use fixed component assumptions for the current layout, so they should not be treated as fully responsive components without additional width guards.
- Text containing tabs is normalized to four spaces prior to wrapping.
- Wrapped lines are tracked as separate logical lines for both rendering and link-zone coordinates.

## Path Segmentation

`segment_line_path` divides a line into path and non-path segments.

Recognized path forms include:

- Paths containing directory separators, such as `src/main.rs`.
- Tilde-prefixed paths, such as `~/work/app/src/main.rs`.
- Standalone filenames with extensions, such as `Cargo.toml`.
- Multi-dot filenames, such as `pcss.config.js`.
- Dotfiles, such as `.env` and `.env.local`.

```rust
pub struct TextSeg<'a> {
	pub text: String,
	pub file_path: Option<&'a str>,
}

pub fn segment_line_path(line: &str) -> Vec<TextSeg<'_>> {
	static RE: LazyLock<Regex> = LazyLock::new(|| {
		Regex::new(
			r#"(?x)
			~?[a-zA-Z0-9_@\-\./]+/[a-zA-Z0-9_@\-\.]+\.[a-zA-Z0-9]{2,5}
			|
			[a-zA-Z0-9_@\-]+(?:\.[a-zA-Z0-9_@\-]+)*\.[a-zA-Z][a-zA-Z0-9]{0,4}
			|
			\.[a-zA-Z][a-zA-Z0-9_\-]*(?:\.[a-zA-Z][a-zA-Z0-9]*)*
		"#,
		)
		.expect("Failed to compile segment_line_path regex")
	});

	let re = &*RE;
	let mut segments = Vec::new();
	let mut last_idx = 0;

	for m in re.find_iter(line) {
		let start = m.start();
		let end = m.end();
		let text = &line[start..end];

		// Reject standalone filenames followed by alphanumeric/hyphen/dot continuation
		if !text.contains('/') && !text.starts_with('.') {
			if let Some(&b) = line.as_bytes().get(end)
				&& (b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
			{
				continue;
			}
		}

		if start > last_idx {
			segments.push(TextSeg { text: line[last_idx..start].to_string(), file_path: None });
		}
		segments.push(TextSeg { text: text.to_string(), file_path: Some(text) });
		last_idx = end;
	}

	if last_idx < line.len() {
		segments.push(TextSeg { text: line[last_idx..].to_string(), file_path: None });
	}

	segments
}
```

Path text styling rules:

- Normal path text uses `STL_TXT_PATH` (or an optional debug color override).
- Hovered path text uses `STL_TXT_PATH_HOVER`, which adds the `Modifier::UNDERLINED` attribute.
- Path links attach a dedicated `UiAction::OpenFile(path_string)` link zone.

## Style Control

Styles are centralized in the TUI style modules:

- `style_consts.rs` for reusable colors and constant styles.
- `style_common.rs` for dynamic style selection.
- `style_text_path` for normal, hovered, and debug-colored file paths.
- `UiExt::x_fg` and `UiExt::x_bg` for applying colors across mutable span collections.
- Direct span style mutation only when the target span range is already known and the change is local to the interaction state.

Hover code should preserve the intended distinction between:

- Marker styling.
- Normal content styling.
- Path styling.
- Group hover styling.
- Selected navigation styling.

A grouped hover normally changes the foreground to `CLR_TXT_HOVER_TO_CLIP`. A path hover uses the underlined path style from `style_text_path`.

## Separators and Line Accounting

Sections commonly end with an empty `Line`.

When a separator is added after interactive content:

- Add the separator to the returned line collection.
- Do not attach a link zone to the separator.
- Advance `LinkZones.current_line` for the separator.
- Set the next section's current line before registering its zones.

Line accounting must use the same logical line sequence for:

- The rendered `Vec<Line>`.
- `LinkZone.line_idx`.
- Scroll calculations.
- Virtualized viewport offsets.
