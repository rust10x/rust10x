# CLI TUI Multi-Zone Scrolling

## Purpose

This guide documents multi-zone scroll coordination, scrollbar math, discrete indicator placement, and viewport management across CLI views.

## Multi-Zone Architecture Overview

Complex TUI screens often display multiple simultaneously visible, independently scrollable regions. For example, a left sidebar for run navigation (`RunsNav`), a middle pane for task navigation (`TasksNav`), and a right content pane for task logs (`TaskContent`).

To support multiple scrollable zones without conflicting scroll offsets or misrouted input, the architecture uses a centralized zone registry combined with dynamic area registration and lifecycle cleanup:

- Typed Zone Identifiers: Each scrollable area is uniquely identified by a variant of the `ScrollIden` enum.
- Centralized Registry: `ScrollZones` maintains a map of `ScrollIden` to `ScrollZone` instances storing each zone's current viewport rectangle and scroll offset.
- Dynamic Area Registration: Views re-register their current rendered `Rect` every frame using `AppState::set_scroll_area`.
- Pointer Hit Testing: Mouse wheel events are routed to whichever scroll zone currently contains the mouse pointer.
- Keyboard Fallback Routing: Keyboard scroll actions can bypass pointer position and target the primary content zone for the active tab when `SCROLL_KEY_MAIN_VIEW` is enabled.
- Inactive Zone Cleanup: Parent views and tab switchers explicitly clear the viewport areas of inactive or hidden zones to prevent inactive zones from intercepting mouse wheel input.

```rust
use ratatui::layout::{Position, Rect};
use std::collections::HashMap;

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

## Multi-Zone Lifecycle and View Coordination

Coordinating multiple scroll zones across tabs and nested views follows strict ownership and cleanup rules:

1. Frame Registration: During its `render` function, every active scrollable component calls `state.set_scroll_area(SCROLL_IDEN, area)` with its allocated layout rectangle.
2. Inactive Area Clearing: When a view switches tabs or hides a sub-view, the parent view must clear the scroll areas of the inactive child components.
3. Compound Cleanup Methods: Composite views provide static cleanup methods (such as `RunTasksView::clear_scroll_idens`) that clear their own zone as well as delegating to nested child view clear functions (such as `TaskView::clear_scroll_idens`).
4. Pointer Hit Resolution: In `process_app_state`, mouse coordinates are tested against all registered areas via `find_zone_for_pos`. If the pointer is over an area whose zone was properly registered, `active_scroll_zone_iden` is set to that zone. Inactive zones whose areas were cleared will never match.
5. Independent Clamping: Each zone computes and updates its own scroll offset via `clamp_scroll` independently. Modifying or clamping one zone has zero impact on neighboring or sibling zones.

## Input Routing

The state processor routes scroll events through two paths:

- Mouse wheel events use the scroll zone containing the current mouse position.
- Keyboard scroll events use the active main content tab when `SCROLL_KEY_MAIN_VIEW` is enabled.

Keyboard actions include:

- Single-line scroll.
- Page scroll (fixed at 5 logical units).
- Scroll to beginning (Home / Shift+Up).
- Scroll to end (End / Shift+Down).

```rust
if let Some(dir) = scroll_dir {
	let mut zone_iden = state.core().active_scroll_zone_iden;

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

## Clamping Calculations

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
- A table passes the number of data rows.

Clamping must happen before calculating visible ranges. Selection adjustments that modify the scroll position must also happen before building the visible slice.

## Rendering Strategies

The current views use several valid scroll strategies.

### Paragraph Offset

`TaskView` and the overview content view build logical lines and render them with `Paragraph::scroll`.

This strategy is appropriate when:

- Link zones need access to the complete logical line collection.
- Sections have variable heights.
- Content is already available in memory.
- A scrollbar can use the total logical line count.

### Visible-Line Virtualization

`RunTasksView` and the overview task section calculate the visible range and build only the rows that can appear in the viewport.

This strategy is appropriate when:

- The data set can be large.
- Row construction is independent.
- The view can calculate a stable logical line count.
- Link-zone offsets can be mapped from logical rows to rendered rows.

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

### List Offset

`RunsNavView` uses `ListState` with a manually controlled offset. Appropriate for uniform list rows where selection and scrolling are maintained separately.

### Table Viewport

`GroupDashView` calculates visible row indexes and renders only the rows in the current range.

## Scrollbar Math and Styling Patterns

The CLI uses two distinct visual patterns for displaying scroll progress and overflow: full Ratatui vertical scrollbars for long multiline views (such as `RunOverviewView` and `TaskView`), and contextual discrete indicator icons for list panes (such as `RunTasksView`).

### Vertical Scrollbar Widget (`Scrollbar` & `ScrollbarState`)

For scrollable paragraphs and log bodies, the TUI configures Ratatui's `Scrollbar` widget attached to the right edge of the content area.

1. Width Reservation: Content rendering reserves horizontal space for the vertical scrollbar track by subtracting 3 columns from the content layout width (`let max_width = area.width - 3;`).
2. Content Size and Position Math: `ScrollbarState` requires `content_length` (the maximum scroll offset range) and `position` (the current scroll offset). The content size represents the amount of content extending beyond a single viewport screen:
   ```rust
   let content_size = line_count.saturating_sub(area.height as usize);
   let mut scrollbar_state = ScrollbarState::new(content_size).position(scroll as usize);
   ```
3. Symbols and Characters:
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

### Discrete Scroll Indicator Icons

In list sidebars (such as task navigation), where a full scrollbar track would consume too much horizontal width, single-cell indicator icons are rendered conditionally in the corners of the list area.

1. Bottom Overflow Math:
   ```rust
   let item_count = tasks_len as u16;
   if item_count.saturating_sub(scroll) > tasks_list_a.height {
   	let bottom_ico = tasks_list_a.x_bottom_right(1, 1);
   	comp::ico_scroll_down().render(bottom_ico, buf);
   }
   ```

2. Top Overflow Math:
   ```rust
   if scroll > 0 && item_count > tasks_list_a.height.saturating_sub(scroll) {
   	let top_ico = tasks_list_a.x_top_right(1, 1);
   	comp::ico_scroll_up().render(top_ico, buf);
   }
   ```

3. Placement Helpers:
   - `area.x_top_right(1, 1)` allocates a 1x1 cell in the top-right corner.
   - `area.x_bottom_right(1, 1)` allocates a 1x1 cell in the bottom-right corner.
   - `ico_scroll_up()` and `ico_scroll_down()` render styled arrow glyphs.

## Scroll Invariants & Checklist

The following invariants must hold:

- The registered scroll `Rect` must match the area passed to the widget or link-zone hit tester.
- The content count passed to `clamp_scroll` must match the logical coordinate system used by rendering.
- Virtualized output must preserve the logical row offset.
- Inactive views must clear their scroll areas.
- A scroll value must be clamped after content size or viewport size changes.
- Scrollable renderers must handle zero-height areas without producing invalid row coordinates.
