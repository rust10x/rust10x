# CLI TUI Interaction & LinkZones

## Purpose

This guide details span-level interactive metadata (`LinkZone`), pointer hit testing, multi-line grouped hovers, overlapping zone precedence, and action dispatch integration.

## LinkZone Metadata

A `LinkZone` connects a rendered span range to a `UiAction`.

### Terminology: LinkZone vs ScrollZone

The CLI codebase maintains two distinct zone systems:

- `ScrollZone` / `ScrollZones`: Tracks viewport bounding boxes and scroll offsets for mouse-wheel hit resolution and keyboard scroll routing.
- `LinkZone` / `LinkZones`: Tracks span-level coordinate ranges within rendered lines to bind click and hover events to executable `UiAction` intents (such as `OpenFile`, `ToClipboardCopy`, or `GoToTask`).

The `LinkZone` name explicitly differentiates span-level interactive action targets from viewport-level scrollable regions (`ScrollZone`).

Each zone stores:

- `line_idx`, the logical line number.
- `span_start`, the first span covered by the zone.
- `span_count`, the number of spans covered.
- `action`, the intent to store when clicked.
- `group_id`, an optional identifier for section-wide hover behavior.

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

## Zone Registration

The standard registration sequence is:

- Set `current_line` to the logical start of the section.
- Build each line and record relative line indexes.
- Register path-specific zones for path spans.
- Register a grouped zone for the content range when the section has a main action.
- Increment `current_line` by the number of rendered lines.
- Increment again when a separator line is appended.
- Set the next section's current line before building its zones.

`push_link_zone` registers an ungrouped zone suitable for a file path or task block. `start_group` and `push_group_zone` register zones that participate in section-wide hover and click behavior.

## Grouped Sections

Grouped zones provide consistent interaction over wrapped multiline sections.

A grouped section normally includes:

- One group identifier for the section.
- A zone for each content span or segment.
- A broader zone covering the complete content range.
- More specific path zones layered over the broader group zone.

The broad group action is commonly `ToClipboardCopy`. The path-specific action is commonly `OpenFile`.

## Hit Testing

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

## Overlapping Zones & Precedence

Zones intentionally overlap when a path is inside a grouped content section.

The views resolve overlap by selecting the matching zone with the smallest `span_count`.

This gives the intended precedence:

- A path-specific zone wins over the full content group.
- A task-specific zone wins over a broader row zone.
- A broad section zone handles all other content in the section.

## Hover Styling and Click Handling

Hover rendering uses two passes:

- Pass 1: checks every zone against the mouse position and tracks the matching zone with the smallest span count.
- Pass 2: reads the selected zone's action and group, applies hover styles to matching spans, and stores the action in `AppState` when clicked.

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

## LinkZone Invariants & Checklist

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
- Clear consumed mouse events when the action is accepted.
