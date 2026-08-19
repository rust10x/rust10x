# CLI TUI Design Patterns

## Purpose

This guide documents the current Ratatui TUI design patterns used by the CLI. It focuses on the control points and contracts needed to build or update views without breaking layout, scrolling, hover behavior, or action dispatch.

The documented patterns cover:

- Span and line construction.
- Rendering function responsibilities.
- Scroll state and viewport management.
- `LinkZone` registration, hit testing, hover styling, and action dispatch.
- Coordinate and state invariants shared by these systems.

The patterns are based on the current implementation under `src/tui`.

## Core Architecture Overview

The TUI separates state management, content construction, interaction metadata collection, and widget rendering into distinct stages:

1. `AppState` stores selection, scroll positions, mouse events, pending actions, and model data.
2. The state processor transforms raw terminal/keyboard/mouse events into state updates.
3. Views register their active scroll areas, calculate clamping, and construct owned `Line<'static>` or `Span<'static>` collections.
4. Builders optionally populate `LinkZones` with action metadata, span offsets, and grouping ids.
5. Views execute a two-pass hover test: pass 1 finds the most specific hovered zone; pass 2 mutates span styles (e.g. hover highlight) and queues clicked actions into `AppState`.
6. Ratatui widgets (`Paragraph`, `Scrollbar`, `Block`) render the styled buffer.
7. The state processor centrally executes queued `UiAction` side effects (clipboard copies, file opens, task navigation).

## Rendering Architecture

The TUI separates state management, content construction, interaction processing, and widget rendering.

The normal rendering flow is:

- `AppState` stores selection, scroll positions, mouse events, pending actions, and model-derived data.
- The state processor converts terminal and application events into state changes.
- A stateful view registers its active scroll area and builds the content required for the current screen.
- Component and facade functions construct owned `Line<'static>` or `Span<'static>` values.
- The view processes hover and click metadata against the constructed lines.
- Ratatui widgets render the final lines, paragraphs, lists, tables, and scrollbars.

A view should not execute application side effects directly. It should store a `UiAction` in `AppState`, then let the state processor perform clipboard operations, file opening, navigation, or other actions.

### Span Extension Helper (`UiExt`)

Mutable span slices and lines implement `UiExt` to enable fluent style mutations during hover passes and width calculations:

```rust
use ratatui::style::Color;
use ratatui::text::{Line, Span};

pub trait UiExt {
	fn x_bg(self, color: Color) -> Self;
	fn x_fg(self, color: Color) -> Self;
	fn x_width(&self) -> u16;
}

impl<'a> UiExt for &mut [Span<'a>] {
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

## Span and Line Rendering Patterns

### Motivation: Direct Span-Level Control

Constructing and managing individual `Span<'static>` instances directly, rather than relying on high-level opaque widgets, provides essential control points:

- Responsive width measurement: Span widths can be computed prior to buffer rendering using `UiExt::x_width`, allowing dynamic truncation, padding, and alignment adjustments based on container bounds.
- Selective styling: Individual token segments (markers, labels, values, file paths, status icons) maintain independent colors and modifiers without string splitting at render time.
- Dynamic hover and interaction states: Mutable span slices (`&mut [Span]`) allow two-pass hover highlighting to update foreground colors, backgrounds, or underline modifiers immediately before passing lines to Ratatui widgets.
- Precise interaction mapping: `LinkZone` hit testing relies on exact span byte offsets and calculated visual widths to resolve clicks down to specific path tokens or broad grouped blocks.

### Owned Render Output

Content builders generally return owned values:

- `Vec<Line<'static>>` for multiline content.
- `Vec<Span<'static>>` for a single row or reusable span group.
- `Line::from(spans)` for assembling a line from styled spans.
- `Span::raw`, `Span::styled`, and style extensions for individual content segments.

Owned output is important because views may mutate styles after content construction. Hover handling changes the foreground, background, or complete style of spans before the lines are rendered.

The main shared component is `ui_for_marker_section_str`. It produces marker-prefixed, wrapped, optionally path-aware content and can register link zones while it builds the lines.

### Function Schemes

Use function names to communicate the amount of state and interaction involved.

- `ui_for_*`

  Builds content without attaching interactive behavior. These functions should be as close to pure builders as possible.

- `ui_for_*_with_hover`

  Builds content and registers `LinkZone` metadata for hover and click handling. The function should not execute the action.

- `render_*`

  Owns view-level orchestration. It prepares layout areas, registers scroll zones, loads data, processes input, renders widgets, and displays indicators.

- `Task::ui_*`

  Facade methods on model types produce reusable span groups for task-specific UI elements such as labels, input, AI status, output, skipped status, and compact task blocks.

- Support functions

  Shared helpers such as `extend_lines`, path segmentation, rectangle extensions, and width calculation should remain independent from application state unless state is required by their purpose.

The responsibility split is:

- Components decide what content and spans exist.
- Views decide where content is placed and how it is scrolled.
- `LinkZones` describes which rendered spans are interactive.
- `AppState` stores the resulting intent.
- The state processor executes the intent.

### Text Formatting Patterns

Consistent text formatting ensures visual stability and predictable alignment across varied terminal dimensions:

- Number and index padding: Use fixed-width padding based on total collection length (for example `text::num_pad_for_len(idx, total_len)`) so list item numbers align vertically.
- Truncation with ellipsis: For single-line constraints, truncate long descriptions with trailing ellipsis (for example `truncate_with_ellipsis(text, max_width, "..")` or `truncate(text, max_width)`) to prevent uncontrolled wrapping.
- Tab normalization: Tab characters (`\t`) must be converted to uniform spaces (typically 4 spaces) before width calculations and text wrapping to prevent coordinate drift.
- Alignment formatting: Format string macros (such as `format!("{label:<width$}")` for left-alignment or right-aligned Paragraphs) guarantee that bounding boxes remain exact.
- Numeric metrics: Duration and cost formatting helper functions produce compact, human-readable strings with fixed or predictable bounds.

### Marker Section Layout

`ui_for_marker_section_str` follows a consistent layout scheme:

- A marker is right-aligned to a minimum width.
- A one-character spacer follows the marker.
- Optional prefix spans follow the spacer.
- Content is wrapped to the remaining width.
- Continuation lines receive blank marker indentation.
- Path segments receive path styling and optional `OpenFile` actions.
- Content segments may receive a grouped action such as `ToClipboardCopy`.
- The line accumulator advances the current link-zone line after the section.

The marker width is controlled by `MARKER_MIN_WIDTH`, currently ten characters. This keeps related sections visually aligned even when marker labels have different lengths.

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

### Wrapping and Width Control

Content width is derived from the available width after subtracting the marker, spacer, and optional prefix widths.

Important width rules:

- The caller must provide a width that includes enough room for the marker and spacer.
- Scrollbar space is commonly reserved by passing a reduced width such as `area.width - 3`.
- Narrow terminal layouts need explicit protection before subtracting fixed widths.
- Builders should prefer `saturating_sub` when a width can be zero or smaller than the expected component width.
- Task facade methods use fixed component assumptions for the current layout, so they should not be treated as fully responsive components without additional width guards.
- Text containing tabs is normalized to four spaces prior to wrapping.
- Wrapped lines are tracked as separate logical lines for both rendering and link-zone coordinates.

A narrow-area review should check every calculation that subtracts a marker width, spacer width, scrollbar width, or fixed component width.

### Path Segmentation

`segment_line_path` divides a line into path and non-path segments.

Recognized path forms include:

- Paths containing directory separators, such as `src/main.rs`.
- Tilde-prefixed paths, such as `~/work/app/src/main.rs`.
- Standalone filenames with extensions, such as `Cargo.toml`.
- Multi-dot filenames, such as `pcss.config.js`.
- Dotfiles, such as `.env` and `.env.local`.

Each segment is represented by owned text and an optional path reference.

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

Rendering uses the segment type to choose the style:

- Path segments use `style_text_path(false, path_color)`.
- Other segments use `STL_SECTION_TXT`.
- Path segments receive `UiAction::OpenFile` link zones when interaction is enabled.
- Non-path content may receive a section-wide grouped action.

The path matcher includes post-filtering to avoid treating model versions and date-like identifiers as filenames.

Path text styling rules:

- Normal path text uses `STL_TXT_PATH` (or an optional debug color override).
- Hovered path text uses `STL_TXT_PATH_HOVER`, which adds the `Modifier::UNDERLINED` attribute.
- Path links attach a dedicated `UiAction::OpenFile(path_string)` link zone.

### Style Control

Styles are centralized in the TUI style modules.

Use:

- `style_consts.rs` for reusable colors and constant styles.
- `style_common.rs` for dynamic style selection.
- `style_text_path` for normal, hovered, and debug-colored file paths.
- `UiExt::x_fg` and `UiExt::x_bg` for applying a color across mutable span collections.
- Direct span style mutation only when the target span range is already known and the change is local to the interaction state.

Hover code should preserve the intended distinction between:

- Marker styling.
- Normal content styling.
- Path styling.
- Group hover styling.
- Selected navigation styling.

A grouped hover normally changes the foreground to `CLR_TXT_HOVER_TO_CLIP`. A path hover uses the underlined path style from `style_text_path`.

### Separators and Line Accounting

Sections commonly end with an empty `Line`.

When a separator is added after interactive content:

- Add the separator to the returned line collection.
- Do not attach a link zone to the separator.
- Advance `LinkZones.current_line` for the separator.
- Set the next section's current line before registering its zones.

`extend_lines` avoids adding anything when the source collection is empty. When requested, it appends one empty line after non-empty content.

Line accounting must use the same logical line sequence for:

- The rendered `Vec<Line>`.
- `LinkZone.line_idx`.
- Scroll calculations.
- Virtualized viewport offsets.

## Scroll Patterns

### Multi-Zone Architecture Overview

Complex TUI screens often display multiple simultaneously visible, independently scrollable regions. For example, a left sidebar for run navigation (`RunsNav`), a middle pane for task navigation (`TasksNav`), and a right content pane for task logs (`TaskContent`).

To support multiple scrollable zones without conflicting scroll offsets or misrouted input, the architecture uses a centralized zone registry combined with dynamic area registration and lifecycle cleanup:

- **Typed Zone Identifiers**: Each scrollable area is uniquely identified by a variant of the `ScrollIden` enum.
- **Centralized Registry**: `ScrollZones` maintains a map of `ScrollIden` to `ScrollZone` instances storing each zone's current viewport rectangle and scroll offset.
- **Dynamic Area Registration**: Views re-register their current rendered `Rect` every frame using `AppState::set_scroll_area`.
- **Pointer Hit Testing**: Mouse wheel events are routed to whichever scroll zone currently contains the mouse pointer.
- **Keyboard Fallback Routing**: Keyboard scroll actions can bypass pointer position and target the primary content zone for the active tab when `SCROLL_KEY_MAIN_VIEW` is enabled.
- **Inactive Zone Cleanup**: Parent views and tab switchers explicitly clear the viewport areas of inactive or hidden zones to prevent inactive zones from intercepting mouse wheel input.

### Scroll State Model

Scroll state is keyed by the typed `ScrollIden` enum.

Each `ScrollZone` stores:

- An optional `Rect` describing the active viewport.
- An optional `u16` scroll position.
- Optional bottom-state metadata reserved for future use.

The common identifiers are:

- `RunsNav`.
- `TasksNav`.
- `TaskContent`.
- `OverviewContent`.
- `GroupDashContent`.

The scroll position is stored in `AppState`, while the view owns the interpretation of what one scroll unit represents. Depending on the view, one unit may represent a line, a task row, a list item, or a table row.

```rust
use ratatui::layout::{Position, Rect};
use std::collections::HashMap;

#[derive(Debug)]
pub(in crate::tui::core) struct ScrollZones {
	pub zones: HashMap<ScrollIden, ScrollZone>,
}

impl ScrollZones {
	pub fn find_zone_for_pos(&self, position: impl Into<Position>) -> Option<ScrollIden> {
		let position = position.into();
		self.zones
			.iter()
			.find(|(_, zone)| zone.area().is_some_and(|area| area.contains(position)))
			.map(|(iden, _)| *iden)
	}
}
```

### Multi-Zone Lifecycle and View Coordination

Coordinating multiple scroll zones across tabs and nested views follows strict ownership and cleanup rules:

1. **Frame Registration**: During its `render` function, every active scrollable component calls `state.set_scroll_area(SCROLL_IDEN, area)` with its allocated layout rectangle.
2. **Inactive Area Clearing**: When a view switches tabs or hides a sub-view, the parent view must clear the scroll areas of the inactive child components. For example:

```rust
impl StatefulWidget for RunMainView {
	type State = AppState;

	fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
		// ... layout tabs and tab_content_a ...
		match selected_tab {
			RunTab::Overview => {
				RunTasksView::clear_scroll_idens(state);
				RunOverviewView.render(tab_content_a, buf, state);
			}
			RunTab::Tasks => {
				RunOverviewView::clear_scroll_idens(state);
				RunTasksView.render(tab_content_a, buf, state);
			}
		}
	}
}
```

3. **Compound Cleanup Methods**: Composite views provide static cleanup methods (e.g. `RunTasksView::clear_scroll_idens`) that clear their own zone as well as delegating to nested child view clear functions (such as `TaskView::clear_scroll_idens`).
4. **Pointer Hit Resolution**: In `process_app_state`, mouse coordinates are tested against all registered areas via `find_zone_for_pos`. If the pointer is over an area whose zone was properly registered, `active_scroll_zone_iden` is set to that zone. Inactive zones whose areas were cleared will never match.
5. **Independent Clamping**: Each zone computes and updates its own scroll offset via `clamp_scroll` independently. Modifying or clamping one zone has zero impact on neighboring or sibling zones.

```rust
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ScrollIden {
	RunsNav,
	TasksNav,
	TaskContent,
	OverviewContent,
	GroupDashContent,
}

#[derive(Debug, Default)]
pub struct ScrollZone {
	area: Option<Rect>,
	scroll: Option<u16>,
	is_bottom: bool,
}
```

### Scroll Lifecycle

A scrollable view should follow this lifecycle:

- Register the visible viewport with `set_scroll_area`.
- Compute the logical content count.
- Call `clamp_scroll` before using the scroll value.
- Build or select the visible content using the clamped value.
- Render the content with the matching viewport offset.
- Render an indicator when content extends beyond the viewport.
- Clear the scroll area when the parent view is inactive or a sibling tab replaces it.

Parent views clear inactive child zones when changing tabs. This prevents mouse-wheel events from targeting a scroll area that is no longer visible.

### Input Routing

The state processor routes scroll events through two paths:

- Mouse wheel events use the scroll zone containing the current mouse position.
- Keyboard scroll events use the active main content tab when `SCROLL_KEY_MAIN_VIEW` is enabled.

Keyboard actions include:

- Single-line scroll.
- Page scroll.
- Scroll to the beginning.
- Scroll to the end.

The current page scroll amount is fixed at five logical units. It is not currently derived from the viewport height.

```rust
// State processor scroll dispatch
if let Some(dir) = scroll_dir {
	let mut zone_iden = state.core().active_scroll_zone_iden;

	// If key scroll and SCROLL_KEY_MAIN_VIEW is active, route to active content tab
	if is_key_scroll && SCROLL_KEY_MAIN_VIEW {
		zone_iden = match state.run_tab() {
			RunTab::Overview => Some(ScrollIden::OverviewContent),
			RunTab::Tasks => Some(ScrollIden::TaskContent),
		};
	}

	if let Some(zone_iden) = zone_iden {
		if scroll_to_end {
			match dir {
				ScrollDir::Up => state.set_scroll(zone_iden, 0),
				ScrollDir::Down => state.set_scroll(zone_iden, u16::MAX),
			}
		} else {
			let amount = if is_page { 5 } else { 1 };
			match dir {
				ScrollDir::Up => { state.core_mut().dec_scroll(zone_iden, amount); }
				ScrollDir::Down => { state.core_mut().inc_scroll(zone_iden, amount); }
			}
		}
	}
}
```

Mouse state is separated into:

- `mouse_evt`, which represents the current event during processing.
- `last_mouse_evt`, which persists the latest mouse position for hover rendering.

Views generally use `last_mouse_evt` for hover detection and `mouse_evt` for immediate click or selection processing.

### Clamping

`clamp_scroll` calculates the maximum scroll as:

- Logical content line or item count.
- Minus the viewport height.
- Saturated at zero.
- Stored as a `u16`.

```rust
pub fn clamp_scroll(&mut self, iden: ScrollIden, line_count: usize) -> u16 {
	let Some(scroll_zone) = self.core.get_zone_mut(&iden) else {
		return 0;
	};
	let area_height = scroll_zone.area().map(|a| a.height).unwrap_or_default();
	let max_scroll = line_count.saturating_sub(area_height as usize) as u16;
	let scroll = scroll_zone.scroll().unwrap_or_default();
	if scroll > max_scroll {
		scroll_zone.set_scroll(max_scroll);
		max_scroll
	} else {
		scroll
	}
}
```

The view must pass a content count that matches its rendering strategy:

- Full paragraph content passes the total line count.
- A list passes the total item count.
- A virtualized task view passes the total logical section line count.
- A table passes the number of data rows, with any header adjustment accounted for by the view.

Clamping must happen before calculating visible ranges. Selection adjustments that modify the scroll position must also happen before building the visible slice.

### Rendering Strategies

The current views use several valid scroll strategies.

#### Paragraph offset

`TaskView` and the overview content view build logical lines and render them with `Paragraph::scroll`.

This strategy is appropriate when:

- Link zones need access to the complete logical line collection.
- Sections have variable heights.
- Content is already available in memory.
- A scrollbar can use the total logical line count.

#### Visible-line virtualization

`RunTasksView` and the overview task section calculate the visible range and build only the rows that can appear in the viewport.

This strategy is appropriate when:

- The data set can be large.
- Row construction is independent.
- The view can calculate a stable logical line count.
- Link-zone offsets can be mapped from logical rows to rendered rows.

Virtualized views must preserve the logical position of the visible content. They may insert top padding or use an explicit logical offset, but link zones must continue to use the same line coordinate system as hit testing.

```rust
let start_idx = scroll as usize;
let end_idx = start_idx.saturating_add(tasks_list_a.height as usize).min(tasks.len());
let mut visible_lines: Vec<Line<'static>> = Vec::with_capacity(end_idx.saturating_sub(start_idx));

for idx in start_idx..end_idx {
	let task = &tasks[idx];
	let mut line = Line::from(task.ui_label(Some(" "), area.width, tasks_len));
	if task_sel_idx == idx {
		line = line.style(style::STL_NAV_ITEM_HIGHLIGHT).x_fg(style::CLR_TXT_BLACK);
	} else {
		let visible_row = (idx - start_idx) as u16;
		if is_mouse_in_nav && state.is_last_mouse_over(tasks_list_a.x_row(visible_row + 1)) {
			line = line.fg(style::CLR_TXT_HOVER);
		}
	}
	visible_lines.push(line);
}

Paragraph::new(visible_lines).render(tasks_list_a, buf);
```

#### List offset

`RunsNavView` uses `ListState` with a manually controlled offset.

This strategy is appropriate when:

- Content consists of uniform list rows.
- Selection and scrolling are maintained separately.
- The view needs Ratatui list rendering but wants to control the visible offset.

#### Table viewport

`GroupDashView` calculates visible row indexes and renders only the rows in the current range.

This strategy is appropriate when:

- Header and data-row heights are known.
- Column layout is stable.
- Each tab can calculate its own visible rows.

The group dashboard currently reuses one scroll identifier across its tabs. This is simple, but changing tabs also reuses the previous tab's scroll position.

### Scroll Indicators

Indicators are currently implemented in individual views.

The TUI uses:

- Scroll arrows placed at the top or bottom-right of a viewport.
- A Ratatui vertical scrollbar for paragraph-based content.
- Visibility checks based on the remaining item or line count.

### Scrollbar Math and Styling Patterns

The CLI uses two distinct visual patterns for displaying scroll progress and overflow: full Ratatui vertical scrollbars for long multiline views (such as `RunOverviewView` and `TaskView`), and contextual discrete indicator icons for list panes (such as `RunTasksView`).

#### Vertical Scrollbar Widget (`Scrollbar` & `ScrollbarState`)

For scrollable paragraphs and log bodies, the TUI configures Ratatui's `Scrollbar` widget attached to the right edge of the content area.

1. **Width Reservation**:
   Content rendering reserves horizontal space for the vertical scrollbar track by subtracting 3 columns from the content layout width:
   ```rust
   let max_width = area.width - 3; // reserve space for vertical scrollbar
   ```

2. **Content Size and Position Math**:
   `ScrollbarState` requires `content_length` (the maximum scroll offset range) and `position` (the current scroll offset). The content size represents the amount of content extending beyond a single viewport screen:
   ```rust
   let content_size = line_count.saturating_sub(area.height as usize);
   let mut scrollbar_state = ScrollbarState::new(content_size).position(scroll as usize);
   ```

3. **Symbols and Characters**:
   The scrollbar overrides the default symbols with explicit Unicode arrows:
   - Top begin symbol: `"▲"` (`\u{25B2}`)
   - Bottom end symbol: `"▼"` (`\u{25BC}`)
   - Track orientation: `ratatui::widgets::ScrollbarOrientation::VerticalRight`

```rust
let content_size = line_count.saturating_sub(area.height as usize);
let mut scrollbar_state = ScrollbarState::new(content_size).position(scroll as usize);

let scrollbar = Scrollbar::default()
	.orientation(ratatui::widgets::ScrollbarOrientation::VerticalRight)
	.begin_symbol(Some("▲"))
	.end_symbol(Some("▼"));
scrollbar.render(area, buf, &mut scrollbar_state);
```

#### Discrete Scroll Indicator Icons

In list sidebars (such as task navigation), where a full scrollbar track would consume too much horizontal width, single-cell indicator icons are rendered conditionally in the corners of the list area.

1. **Bottom Overflow Math**:
   Down arrow icon appears when the unviewed items below the current viewport exceed the viewport height:
   ```rust
   let item_count = tasks_len as u16;
   if item_count.saturating_sub(scroll) > tasks_list_a.height {
   	let bottom_ico = tasks_list_a.x_bottom_right(1, 1);
   	comp::ico_scroll_down().render(bottom_ico, buf);
   }
   ```

2. **Top Overflow Math**:
   Up arrow icon appears when the list is scrolled down from the top and there are remaining items above:
   ```rust
   if scroll > 0 && item_count > tasks_list_a.height.saturating_sub(scroll) {
   	let top_ico = tasks_list_a.x_top_right(1, 1);
   	comp::ico_scroll_up().render(top_ico, buf);
   }
   ```

3. **Placement Helpers**:
   The icon positions use rectangle extension helpers:
   - `area.x_top_right(1, 1)` allocates a 1x1 cell in the top-right corner.
   - `area.x_bottom_right(1, 1)` allocates a 1x1 cell in the bottom-right corner.
   - `ico_scroll_up()` and `ico_scroll_down()` render styled arrow glyphs.

A future shared helper could centralize indicator placement, but the current implementation gives each view control over whether headers, separators, or reserved scrollbar columns affect the calculation.

### Scroll Invariants

The following invariants must hold:

- The registered scroll `Rect` must match the area passed to the widget or link-zone hit tester.
- The content count passed to `clamp_scroll` must match the logical coordinate system used by rendering.
- A line index must not be shifted without shifting every related link-zone index.
- Virtualized output must preserve the logical row offset.
- Inactive views must clear their scroll areas.
- A scroll value must be clamped after content size or viewport size changes.
- Scrollable renderers must handle zero-height areas without producing invalid row coordinates.

## LinkZone Pattern

### LinkZone Metadata

A `LinkZone` connects a rendered span range to a `UiAction`.

#### Terminology: LinkZone vs ScrollZone

The CLI codebase maintains two distinct zone systems:

- `ScrollZone` / `ScrollZones`: Tracks viewport bounding boxes and scroll offsets for mouse-wheel hit resolution and keyboard scroll routing.
- `LinkZone` / `LinkZones`: Tracks span-level coordinate ranges within rendered lines to bind click/hover events to executable `UiAction` intents (such as `OpenFile`, `ToClipboardCopy`, or `GoToTask`).

The `LinkZone` name explicitly differentiates span-level interactive action targets from viewport-level scrollable regions (`ScrollZone`).

Each zone stores:

- `line_idx`, the logical line number.
- `span_start`, the first span covered by the zone.
- `span_count`, the number of spans covered.
- `action`, the intent to store when clicked.
- `group_id`, an optional identifier for section-wide hover behavior.

`LinkZones` is the construction-time accumulator. It stores the current logical line and the zones created during content construction.

```rust
#[derive(Debug, Clone)]
pub struct LinkZone {
	pub line_idx: usize,
	pub span_start: usize,
	pub span_count: usize,
	pub action: UiAction,
	pub group_id: Option<u32>,
}

#[derive(Debug, Default, Clone)]
pub struct LinkZones {
	current_line: usize,
	zones: Vec<LinkZone>,
	next_group_id: u32,
}

impl LinkZones {
	pub fn set_current_line(&mut self, current_line: usize) {
		self.current_line = current_line;
	}

	pub fn inc_current_line_by(&mut self, amount: usize) {
		self.current_line += amount;
	}

	pub fn push_link_zone(&mut self, rel_line_idx: usize, span_start: usize, span_count: usize, action: UiAction) {
		let line_idx = self.current_line + rel_line_idx;
		self.zones.push(LinkZone { line_idx, span_start, span_count, action, group_id: None });
	}

	pub fn start_group(&mut self) -> u32 {
		let id = self.next_group_id;
		self.next_group_id = self.next_group_id.wrapping_add(1);
		id
	}

	pub fn push_group_zone(
		&mut self,
		rel_line_idx: usize,
		span_start: usize,
		span_count: usize,
		group_id: u32,
		action: UiAction,
	) {
		let line_idx = self.current_line + rel_line_idx;
		self.zones.push(LinkZone { line_idx, span_start, span_count, action, group_id: Some(group_id) });
	}

	pub fn into_zones(self) -> Vec<LinkZone> {
		self.zones
	}
}
```

### Zone Registration

The standard registration sequence is:

- Set `current_line` to the logical start of the section.
- Build each line and record relative line indexes.
- Register path-specific zones for path spans.
- Register a grouped zone for the content range when the section has a main action.
- Increment `current_line` by the number of rendered lines.
- Increment again when a separator line is appended.
- Set the next section's current line before building its zones.

`push_link_zone` registers an ungrouped zone. It is suitable for:

- A file path that opens in an editor.
- A task block that selects a task.
- A small, precise interactive span range.

`start_group` creates a group identifier. `push_group_zone` registers a zone that participates in section-wide hover and click behavior.

### Grouped Sections

Grouped zones provide consistent interaction over wrapped multiline sections.

A grouped section normally includes:

- One group identifier for the section.
- A zone for each content span or segment.
- A broader zone covering the complete content range.
- More specific path zones layered over the broader group zone.

The broad group action is commonly `ToClipboardCopy`. The path-specific action is commonly `OpenFile`.

The section remains interactive across wrapped lines because every logical line receives the required group zone.

### Hit Testing

`LinkZone::is_mouse_over` performs hit testing using:

- The reference viewport `Rect`.
- The current scroll offset.
- The persistent mouse event.
- The complete span list for the logical line.

The hit-test process is:

- Reject zones above the visible top.
- Reject zones at or below the visible bottom.
- Calculate the visible row from `line_idx - scroll`.
- Measure the spans before the zone to calculate its horizontal offset.
- Measure the zone spans to calculate its width.
- Build a one-row `Rect` for the zone.
- Return the mutable zone span slice when the mouse is inside that rectangle.

The caller must provide the same reference area, line indexing, span ordering, and scroll value used when the lines were built.

```rust
impl LinkZone {
	pub fn is_mouse_over<'a>(
		&self,
		ref_area: Rect,
		scroll: u16,
		mouse_evt: Option<MouseEvt>,
		spans: &'a mut [Span<'static>],
	) -> Option<&'a mut [Span<'static>]> {
		let mouse_evt = mouse_evt?;
		let line_idx = self.line_idx;
		let scroll_usize = scroll as usize;
		let visible_top = scroll_usize;
		let visible_bottom = scroll_usize + ref_area.height as usize;

		if line_idx < visible_top || line_idx >= visible_bottom {
			return None;
		}

		let before_spans = spans.get(0..self.span_start)?;
		let before_width = before_spans.x_width();
		let zone_spans = spans.get_mut(self.span_start..self.span_start + self.span_count)?;
		let visible_row = (line_idx - scroll_usize) as u16;
		let zone_area = Rect {
			x: ref_area.x + before_width,
			y: ref_area.y + visible_row,
			width: zone_spans.x_width(),
			height: 1,
		};

		if mouse_evt.is_over(zone_area) {
			Some(zone_spans)
		} else {
			None
		}
	}

	pub fn spans_slice_mut<'a>(&self, spans: &'a mut [Span<'static>]) -> Option<&'a mut [Span<'static>]> {
		spans.get_mut(self.span_start..self.span_start + self.span_count)
	}
}
```

### Overlapping Zones

Zones intentionally overlap when a path is inside a grouped content section.

The views resolve overlap by selecting the matching zone with the smallest `span_count`.

This gives the intended precedence:

- A path-specific zone wins over the full content group.
- A task-specific zone wins over a broader row zone.
- A broad section zone handles all other content in the section.

This precedence rule depends on zone ranges accurately describing the rendered spans. Adding a new broad zone without considering its span count can change which action wins.

### Hover Styling

Hover rendering uses two passes.

The first pass:

- Checks every zone against the mouse position.
- Tracks the matching zone with the smallest span count.
- Does not permanently change unrelated spans.

The second pass:

- Reads the selected zone's action and group.
- Applies group styling to every zone in the selected group.
- Applies path styling only to the selected ungrouped path zone.
- Stores the action on mouse release.
- Clears mouse events when appropriate to avoid stale hover behavior on the next screen.

Grouped hover normally updates the foreground color of every span in the group. Path hover applies the path hover style, including its underline modifier.

```rust
let zones = link_zones.into_zones();

// Pass 1: detect most specific hovered zone (minimum span_count)
let mut hovered_idx: Option<usize> = None;
let mut min_span_count = usize::MAX;

for (i, zone) in zones.iter().enumerate() {
	if let Some(line) = all_lines.get_mut(zone.line_idx)
		&& zone.is_mouse_over(area, scroll, state.last_mouse_evt(), &mut line.spans).is_some()
		&& zone.span_count < min_span_count
	{
		min_span_count = zone.span_count;
		hovered_idx = Some(i);
	}
}

// Pass 2: apply hover styling and dispatch clicked action
if let Some(i) = hovered_idx {
	let action = zones[i].action.clone();
	let group_id = zones[i].group_id;

	match group_id {
		Some(gid) => {
			for z in zones.iter().filter(|z| z.group_id == Some(gid)) {
				if let Some(line) = all_lines.get_mut(z.line_idx)
					&& let Some(hover_spans) = z.spans_slice_mut(&mut line.spans)
				{
					for span in hover_spans {
						span.style.fg = Some(style::CLR_TXT_HOVER_TO_CLIP);
					}
				}
			}
		}
		None => {
			if let Some(line) = all_lines.get_mut(zones[i].line_idx)
				&& let Some(hover_spans) = zones[i].spans_slice_mut(&mut line.spans)
			{
				for span in hover_spans {
					span.style = style::style_text_path(true, None);
				}
			}
		}
	}

	if state.is_mouse_up_only() && state.is_last_mouse_over(area) {
		state.set_action(action);
		state.clear_mouse_evts(true);
	}
}
```

### Action Dispatch

Views do not execute `UiAction` values.

The dispatch sequence is:

- A view detects a click.
- The view calls `state.set_action(action)`.
- `AppState` triggers a redraw.
- The state processor reads and clones the pending action.
- The state processor performs the side effect.
- The action is cleared after handling.

Current actions include:

- `ToClipboardCopy`, handled through the application's clipboard and popup flow.
- `OpenFile`, handled through the automatic editor selection flow.
- `GoToTask`, handled by switching to the task tab and allowing the task view to select the target task.

This keeps side effects centralized and allows components to remain reusable.

### LinkZone Invariants

The following invariants must hold:

- `line_idx` uses logical content coordinates, not only the currently visible row.
- `span_start` and `span_count` refer to the exact span vector rendered for that line.
- Group zones and path zones must use the same line and span layout.
- Separators must not inherit zones from the previous section.
- The hit-test scroll value must equal the widget's scroll value.
- Virtualized views must translate logical line indexes consistently.
- The reference area passed to hit testing must match the actual content viewport.
- The most specific matching zone must remain deterministic.
- Actions should be cloned into state and executed outside the rendering function.

## Implementation Checklist

### Span and Function Checklist

- Return owned `Line<'static>` or `Span<'static>` values when hover mutation or later composition is required.
- Keep content builders separate from stateful render functions.
- Use centralized styles rather than embedding repeated color definitions.
- Preserve marker width, spacer width, prefix width, and scrollbar reservations in width calculations.
- Guard fixed-width subtraction for narrow terminals.
- Use the same wrapped line collection for rendering and interaction coordinates.
- Advance line accounting after every section and separator.
- Use path segmentation when file references should be styled or opened.

### Scroll Checklist

- Register the scroll area every time the view is rendered.
- Clear scroll areas for inactive tabs and child views.
- Clamp after content and viewport sizes are known.
- Keep the logical content count consistent with the rendering strategy.
- Apply selection-driven scroll changes before calculating the visible slice.
- Use the same scroll value for rendering and link-zone hit testing.
- Render indicators using the same visible-height assumptions as the content.
- Handle empty and zero-height layouts safely.

### LinkZone Checklist

- Set the current logical line before registering zones.
- Use relative line indexes only within the current section.
- Create group IDs for multiline section-wide actions.
- Register path zones after the path spans have been added.
- Keep separators outside all interactive ranges.
- Resolve overlap by shortest matching span range.
- Apply hover styling after detection, in a separate pass.
- Store actions in `AppState` instead of executing them in the view.
- Clear consumed mouse events when the action is accepted.
- Add tests for scrolling, wrapping, overlapping zones, and virtualized rows.

## Testing Priorities

The current source includes path-segmentation tests. Additional focused tests would improve confidence in the TUI contracts.

Recommended tests include:

- Marker sections with empty content.
- Marker sections with tabs and multiline wrapping.
- Marker sections at minimum terminal widths.
- Correct continuation indentation after wrapping.
- Scroll clamping at zero, at the maximum, and after viewport changes.
- Link hit testing before and after scrolling.
- Link hit testing on the first and last visible rows.
- Grouped multiline hover behavior.
- Path-zone precedence over a grouped section zone.
- Unicode span width calculations.
- Virtualized task rows with non-zero scroll offsets.
- Separator lines that must not trigger the preceding section action.
- Scroll-zone clearing when switching tabs or hiding a nested view.

## Current Tradeoffs and Extension Points

The current architecture favors explicit control over abstraction.

Advantages include:

- Components can construct exactly the span layout required by a view.
- Views can choose the most appropriate scrolling strategy.
- Actions remain centralized and testable.
- Grouped zones support multiline content without changing model types.
- Styles and path detection are reusable across logs, pins, errors, input, output, and AI sections.

Current costs include:

- Hover detection and dispatch are duplicated across several views.
- Scroll indicator calculations are repeated.
- Link-zone line offsets are manually maintained.
- Fixed-width components require narrow-terminal validation.
- Page scrolling is fixed rather than viewport-relative.
- A shared scroll identifier can intentionally or unintentionally share position across tabs.

Potential future abstractions should preserve the existing responsibilities:

- A shared scroll viewport helper could centralize clamping, visible ranges, and indicators.
- A shared link-zone dispatcher could centralize specificity selection and hover application.
- A logical line builder could own line offsets and separator accounting.
- Focused coordinate tests should be added before centralizing behavior, because the current correctness depends on exact line and span mappings.
