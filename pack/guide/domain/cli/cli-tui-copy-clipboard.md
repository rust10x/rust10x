# CLI TUI Click-to-Copy with Auto-Dismiss Popup

## Purpose

This guide details the end-to-end pattern for making a region of the TUI clickable so that a click copies its content to the system clipboard and shows a popup that dismisses itself after a short delay.

The pattern is the integration point of three capabilities that are each documented separately:

- Span-level interaction targets and hit testing, see `pack/guide/domain/cli/cli-tui-interaction.md`.

- Deferred copy target resolution, see `pack/guide/domain/cli/cli-tui-interaction-deferred-action.md`.

- Demand-driven timed rendering for auto-dismiss, see `pack/guide/domain/cli/cli-tui-reactive-ping.md`.

This guide focuses on how those three connect into one user-visible behavior. It does not redefine the zone model, the resolver schema, or the ping loop.

## Overview

The behavior has six stages:

- A view builds its lines and registers a grouped `ActionZone` over the copyable region with a `UiAction::ToClipboardCopy(..)` action.

- On mouse-up over that region, the view stores the action in `AppState` and clears the consumed mouse events.

- The state processor picks up the queued action during its next cycle.

- The processor resolves the content (eagerly from the payload, or deferred from the model) and writes it to the system clipboard.

- The processor shows a timed `PopupView` that reports success or failure.

- `should_be_pinged` returns `true` while the timed popup is alive, so the reactive ping keeps ticking until the popup expires and is cleared.

```text
+----------------+    click     +---------------------+
|  Copy Region   | -----------> |  state.set_action   |
| (grouped zone) |              | ToClipboardCopy(..) |
+----------------+              +----------+----------+
                                           |
                              next process cycle
                                           v
+----------------+  resolve   +---------------------+
|   Clipboard    | <---------- |   State Processor   |
+----------------+             +----------+----------+
                                           |
                              set timed popup
                                           v
+----------------+   ping      +---------------------+
| Auto-dismiss   | <---------- |  should_be_pinged   |
+----------------+             +---------------------+
```

## Step 1: Build the Copy Region

A copy region is a rendered section whose content spans are grouped under one identifier. A `ui_for_*_with_hover` builder renders the section and registers zones against a mutable `ActionZones` accumulator.

The section builder receives the accumulator and the action to store when the region is clicked. The builder registers a broad group zone covering the content range, plus narrower path zones for any embedded paths (which take precedence on hover, see the interaction guide).

```rust
pub fn ui_for_pins_with_hover<'a>(
	pins: impl IntoIterator<Item = &'a Pin>,
	max_width: u16,
	action_zones: &mut ActionZones,
	path_color: Option<Color>,
) -> Vec<Line<'static>> {
	let mut all_lines: Vec<Line<'static>> = Vec::new();

	for pin in pins {
		let marker_style = style::STL_PIN_MARKER;
		let content = pin.content.clone().unwrap_or_else(|| "No content".to_string());

		// Register the section-wide copy zone for this pin's content.
		let lines = comp::ui_for_marker_section_str(
			&content,
			("Pin:", marker_style),
			max_width,
			None,
			Some(action_zones),
			Some(UiAction::ToClipboardCopy(CopyTarget::Pin(pin.id))),
			path_color,
		);

		all_lines.extend(lines);

		// Add a separator line (no zones attached).
		all_lines.push(Line::default());
		action_zones.inc_current_line_by(1);
	}

	all_lines
}
```

Key points:

- The group zone is registered by the section builder, which owns the exact span layout of the rendered lines. Do not register the broad zone from the caller, since the caller does not know the span offsets.

- The separator line is appended after the content but no zones are attached to it, so it must be counted with `inc_current_line_by(1)` so the next section's zones line up.

- When a caller also registers path zones, those narrower zones win on hover because hit testing prefers the smallest `span_count`.

## Step 2: Encode the Copy Action

`UiAction::ToClipboardCopy` is the intent stored in the zone. It carries the copy payload in one of two forms:

- Eager: a fully materialized `String`, used when content cannot be mapped to a durable model entity (for example ephemeral or statically generated text).

- Deferred: a lightweight `CopyTarget` descriptor (`CopyTarget::Pin(id)`, `CopyTarget::Item(id, prop)`, and so on), used when content lives in the model.

Prefer the deferred form for model-backed content because the zone is rebuilt every render pass, so a lightweight descriptor avoids cloning large strings every frame. See `pack/guide/domain/cli/cli-tui-interaction-deferred-action.md` for the `CopyTarget` schema and rationale.

```rust
// Eager: payload captured at render time.
let action = UiAction::ToClipboardCopy(CopyTarget::Raw(rendered_text.clone()));

// Deferred: payload resolved from the model at click time.
let action = UiAction::ToClipboardCopy(CopyTarget::Item(item_id, ItemProp::Output));
```

## Step 3: Detect the Click

The owning view resolves hover and click in two passes over the registered zones. Pass one finds the most specific hovered zone (smallest `span_count`). Pass two applies hover styling to the matching group and, when the mouse-up lands inside the viewport, stores the action in `AppState`.

```rust
let zones = action_zones.into_zones();

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
			// Highlight every zone in the group so the whole section reads as one target.
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

Notes:

- Store a cloned action in state; never execute the copy inside the render function.

- Clear the consumed mouse events so the click is not dispatched again on the next tick.

- The `scroll` value passed to `is_mouse_over` must equal the value used to render the lines.

## Step 4: Resolve and Copy in the State Processor

The queued action is handled during the next event-processing cycle. The processor resolves the payload into a canonical string and writes it to the system clipboard, initializing the clipboard lazily on first use.

```rust
UiAction::ToClipboardCopy(target) => {
	// Resolve deferred targets from the model; eager Raw payloads pass through.
	let resolved = state.resolve_copy_target(&target);

	// Lazily create the clipboard handle and reuse it across calls.
	let ensure_clipboard: Result<(), String> = if state.core().clipboard.is_some() {
		Ok(())
	} else {
		match arboard::Clipboard::new() {
			Ok(cb) => {
				state.core_mut().clipboard = Some(cb);
				Ok(())
			}
			Err(err) => Err(format!("Clipboard init error: {err}")),
		}
	};

	let mut is_err = false;
	let popup_msg = match (resolved, ensure_clipboard) {
		(Err(err), _) => {
			is_err = true;
			format!("Copy failed: {err}")
		}
		(Ok(_), Err(msg)) => {
			is_err = true;
			msg
		}
		(Ok(content), Ok(())) => {
			if let Some(cb) = state.core_mut().clipboard.as_mut() {
				match cb.set_text(content) {
					Ok(()) => "Copied to clipboard".to_string(),
					Err(err) => {
						is_err = true;
						format!("Clipboard error: {err}")
					}
				}
			} else {
				is_err = true;
				"Clipboard unavailable".to_string()
			}
		}
	};

	state.set_popup(PopupView {
		content: popup_msg,
		mode: PopupMode::Timed(Duration::from_millis(1000)),
		is_err,
	});
	state.clear_action();
}
```

Notes:

- Never panic during resolution; return the failure message through the popup.

- Clear the action when done so the same `UiAction` is not replayed.

## Step 5: Show the Auto-Dismiss Popup

The feedback popup uses `PopupMode::Timed(duration)`, which makes it self-expiring. Setting the popup records the start timestamp used by the expiry check.

```rust
#[derive(Debug, Clone)]
pub enum PopupMode {
	/// Disappears automatically after the given duration.
	Timed(Duration),

	/// Stays on screen until dismissed by the user (Esc or click 'x').
	User,
}

pub struct PopupView {
	pub content: String,
	pub mode: PopupMode,
	pub is_err: bool,
}

impl AppState {
	pub fn set_popup(&mut self, popup: PopupView) {
		self.core.popup_start_us = Some(self.core.time);
		self.core.popup = Some(popup);
		self.trigger_redraw();
	}

	pub fn clear_popup(&mut self) {
		self.core.popup = None;
		self.core.popup_start_us = None;
		self.trigger_redraw();
	}
}
```

The popup overlay renders the content centered with a bounded box, and colors the border and text by `is_err`:

```rust
let (txt_style, border_style) = if popup.is_err {
	(style::STL_SECTION_MARKER_ERR, style::CLR_TXT_RED)
} else {
	(style::CLR_TXT_HOVER_TO_CLIP.into(), style::CLR_TXT_WHITE)
};
```

Choose the duration by feedback importance: a short confirmation (`1000` ms) for a success, and a slightly longer duration (`2000` to `3000` ms) when the message includes a path or a cause.

## Step 6: Drive Auto-Dismiss with the Reactive Ping

A timed popup must keep the UI rendering until it expires, otherwise nothing would tick the clock and clear it when idle. This is handled by two pieces already documented in `pack/guide/domain/cli/cli-tui-reactive-ping.md`.

First, `should_be_pinged` reports demand while a timed popup is alive:

```rust
pub fn should_be_pinged(&self) -> bool {
	self.running_tick_count().is_some()
		|| self.popup().is_some_and(|p| p.is_timed())
		|| matches!(self.stage(), AppStage::Installing | AppStage::Installed)
}
```

Second, the state processor expires the popup once the elapsed time passes the duration:

```rust
if let Some(PopupMode::Timed(duration)) = state.popup().map(|p| &p.mode)
	&& let Some(start) = state.core().popup_start_us
	&& state.core().time.saturating_sub(start) >= duration.as_micros() as i64
{
	state.clear_popup();
}
```

Together these mean: showing a timed popup activates the ping loop automatically, and the loop deactivates itself once the popup clears and no other demand remains.

## Eager vs Deferred Copy Payloads

| Aspect | Eager `CopyTarget::Raw(String)` | Deferred `CopyTarget::Entity/Item/..` |
| --- | --- | --- |
| Per-frame cost | Clones the full payload each render | Copies a small descriptor |
| Content source | The rendered or captured text | Authoritative model storage |
| Staleness | Can become stale while mounted | Always resolved at click time |
| Formatting | May capture view-specific wrapping or prefixes | Resolves raw canonical content |
| When to use | Ephemeral or static text | Anything addressable by a model ID |

Prefer the deferred form whenever the content maps to a durable model entity. Fall back to `CopyTarget::Raw` only when there is no stable identifier to resolve against.

## Invariants Checklist

- The section builder owns group-zone registration so `span_start` and `span_count` match the actual rendered spans.

- Separator lines carry no zones but must be counted with `inc_current_line_by(1)`.

- The `scroll` and reference `area` passed to hit testing match the widget's render values.

- The most specific matching zone wins, so path zones layered over a content group behave correctly.

- Actions are cloned into `AppState` and executed in the state processor, never during render.

- Consumed mouse events are cleared once the action is accepted.

- Clipboard initialization is lazy and the handle is reused.

- Copy resolution never panics; failures surface through a timed error popup.

- A timed popup always sets `popup_start_us` and is covered by `should_be_pinged`, so it always auto-dismisses.
