# CLI TUI Deferred Action Resolution

## Purpose

This guide details the deferred action resolution architecture for Ratatui TUI interactions. It explains how semantic target identifiers replace eager string allocations in render loops and describes the lifecycle for querying authoritative data upon user interaction.

## Motivation & Problem Statement

In a terminal user interface, rendering cycles occur frequently, triggered by ticks, keystrokes, mouse moves, and model events. When interactive components eagerly construct and store full data payloads (such as large formatted strings in clipboard actions), several issues arise:

- Unnecessary memory allocations: Strings and formatted buffers are cloned every frame for every interactive zone, even when the user never clicks them.
- Visual formatting drift: Content rendered to the terminal is often truncated, line-wrapped, or styled with prefixes. Capturing what was rendered can inadvertently capture view formatting rather than raw canonical data.
- Stale data: If an entity updates while a view remains mounted, an eagerly constructed payload in an interaction zone can become out of date.

Deferred action resolution solves these problems by decoupling interaction targeting from data retrieval. Instead of holding raw content, an `ActionZone` holds a lightweight semantic descriptor. Data extraction occurs only when the action is explicitly dispatched.

## Semantic Target Schema

Interactive targets are represented by lightweight enum descriptors referencing entity IDs and specific property targets.

```rust
use crate::model::Id;

/// Represents the semantic data target to resolve for clipboard or inspection actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopyTarget {
	/// Resolves an entity property from the primary entity model.
	Entity(Id, EntityProp),
	/// Resolves a sub-entity or record property.
	Item(Id, ItemProp),
	/// Resolves an error record by ID.
	Error(Id),
	/// Resolves a log or marker entry by ID.
	Log(Id),
	/// Resolves a persistent note or pin record by ID.
	Pin(Id),
	/// Fallback for static or dynamically generated ephemeral text.
	Raw(String),
}

/// Identifies a specific property on a primary entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityProp {
	Input,
	Output,
	Summary,
}

/// Identifies a specific property on a child item or task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemProp {
	Input,
	Output,
	SkipReason,
	Error,
}
```

By wrapping target descriptors within `UiAction`, action variants remain compact:

```rust
#[derive(Debug, Clone)]
pub enum UiAction {
	Quit,
	Redo,
	ToClipboardCopy(CopyTarget),
	OpenFile(String),
	GoToItem { item_id: Id },
	// ...
}
```

## Registering Semantic Targets in Components

Component builders (`ui_for_*_with_hover`) register `ActionZone` instances that store `CopyTarget` variants rather than cloned strings.

```rust
pub fn ui_for_item_output_with_hover(
	item_id: Id,
	output_preview: &str,
	max_width: u16,
	action_zones: &mut ActionZones,
) -> Vec<Line<'static>> {
	let marker_style = style::STL_SECTION_MARKER;
	let line_idx = action_zones.current_line();

	// Register semantic descriptor without cloning full output content
	let action = UiAction::ToClipboardCopy(CopyTarget::Item(item_id, ItemProp::Output));

	let lines = comp::ui_for_marker_section_str(
		output_preview,
		("Output:", marker_style),
		max_width,
		None,
		Some(action_zones),
		Some(action),
		None,
	);

	lines
}
```

This pattern provides significant advantages:

- Minimal per-frame cost: Constructing `CopyTarget::Item(id, prop)` is a trivial integer and enum copy.
- Scalability: Long multiline texts, large JSON payloads, or massive log outputs do not incur heap allocations during rendering passes.

## Deferred Resolution Lifecycle

The complete dispatch and extraction lifecycle follows a strict sequence:

1. Interaction Detection: The view matches a mouse click or keyboard shortcut to an `ActionZone` holding a `UiAction::ToClipboardCopy(CopyTarget)`.
2. Action Queuing: The view records the action in application state via `state.set_action(action)`.
3. State Processing: During the next event processing cycle, the state processor receives the queued `UiAction`.
4. Canonical Data Extraction: The state processor delegates to a resolver function that queries the model storage (such as `ModelManager` or entity BMCs) using the entity ID in the descriptor.
5. Side Effect Execution: The resolved canonical string is written to the system clipboard, and a feedback popup is displayed.

```text
+-------------------+      Click / Key       +----------------------+
|   Rendered View   | ---------------------> |   state.set_action   |
+-------------------+                        +----------------------+
                                                        |
                                                        v
+-------------------+      Model Query       +----------------------+
|   Storage / BMC   | <--------------------- |   State Processor    |
+-------------------+                        +----------------------+
          |                                             |
          | Authoritative Content                       | Write to Clipboard
          v                                             v
+-------------------+                        +----------------------+
| Canonical String  | ---------------------> |   System Clipboard   |
+-------------------+                        +----------------------+
```

## Resolver Implementation

The data resolution logic is centralized in the state processor or a dedicated action handler:

```rust
impl AppState {
	/// Resolves a CopyTarget into its authoritative text representation.
	pub fn resolve_copy_target(&self, target: &CopyTarget) -> Result<String, String> {
		let mm = self.mm();

		match target {
			CopyTarget::Item(item_id, ItemProp::Input) => {
				let item = ItemBmc::get(mm, *item_id).map_err(|e| e.to_string())?;
				ItemBmc::get_input_for_display(mm, &item)
					.map_err(|e| e.to_string())?
					.ok_or_else(|| "No input content available".to_string())
			}

			CopyTarget::Item(item_id, ItemProp::Output) => {
				let item = ItemBmc::get(mm, *item_id).map_err(|e| e.to_string())?;
				ItemBmc::get_output_for_display(mm, &item)
					.map_err(|e| e.to_string())?
					.ok_or_else(|| "No output content available".to_string())
			}

			CopyTarget::Item(item_id, ItemProp::SkipReason) => {
				let item = ItemBmc::get(mm, *item_id).map_err(|e| e.to_string())?;
				item.end_skip_reason.ok_or_else(|| "No skip reason recorded".to_string())
			}

			CopyTarget::Log(log_id) => {
				let log = LogBmc::get(mm, *log_id).map_err(|e| e.to_string())?;
				log.message.ok_or_else(|| "Log entry has no message".to_string())
			}

			CopyTarget::Pin(pin_id) => {
				let pin = PinBmc::get(mm, *pin_id).map_err(|e| e.to_string())?;
				pin.content.ok_or_else(|| "Pin has no content".to_string())
			}

			CopyTarget::Error(err_id) => {
				let err_rec = ErrBmc::get(mm, *err_id).map_err(|e| e.to_string())?;
				err_rec.content.ok_or_else(|| "Error record has no content".to_string())
			}

			CopyTarget::Raw(text) => Ok(text.clone()),

			_ => Err("Unsupported target resolution".to_string()),
		}
	}
}
```

## Error Handling & Feedback

When resolving deferred targets, data retrieval may occasionally fail (for instance, if an entity was deleted or trimmed from memory storage).

Resolution error handling rules:

- Never panic during action resolution.
- Catch database and model lookup errors gracefully.
- Present failure feedback to the user via timed error popups (`PopupView { is_err: true, .. }`).
- Present clear confirmation upon success, indicating what was copied.

```rust
let (content, is_err) = match state.resolve_copy_target(&target) {
	Ok(text) => match clipboard.set_text(text) {
		Ok(()) => ("Copied to clipboard".to_string(), false),
		Err(err) => (format!("Clipboard error: {err}"), true),
	},
	Err(err) => (format!("Copy failed: {err}"), true),
};

state.set_popup(PopupView {
	content,
	mode: PopupMode::Timed(Duration::from_millis(1500)),
	is_err,
});
```

## Architecture Invariants

- Component views must only construct lightweight `CopyTarget` descriptors.
- Views must not trigger direct database reads or clipboard mutations during render.
- Canonical resolution must query authoritative storage rather than visual span buffers.
- Fallback `CopyTarget::Raw` must only be used when content cannot be mapped to a durable model entity ID.
