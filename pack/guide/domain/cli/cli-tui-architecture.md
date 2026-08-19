# CLI TUI Architecture & Flow

## Purpose

This guide documents the high-level application lifecycle, rendering pipeline, function naming conventions, and action dispatch flow used by the Ratatui TUI in the CLI.

It provides the foundational principles required before implementing view layouts, scroll zones, or interactive link metadata.

## Core Architecture Overview

The TUI separates state management, content construction, interaction metadata collection, and widget rendering into distinct stages:

1. `AppState` stores selection, scroll positions, mouse events, pending actions, and model data.
2. The state processor transforms raw terminal, keyboard, and mouse events into state updates.
3. Views register their active scroll areas, calculate clamping, and construct owned `Line<'static>` or `Span<'static>` collections.
4. Builders optionally populate `LinkZones` with action metadata, span offsets, and grouping identifiers.
5. Views execute a two-pass hover test; pass 1 finds the most specific hovered zone; pass 2 mutates span styles (such as hover highlight) and queues clicked actions into `AppState`.
6. Ratatui widgets (`Paragraph`, `Scrollbar`, `Block`) render the styled buffer.
7. The state processor centrally executes queued `UiAction` side effects (clipboard copies, file opens, task navigation).

## Rendering Architecture

The normal rendering flow proceeds through well-defined stages:

- `AppState` stores selection, scroll positions, mouse events, pending actions, and model-derived data.
- The state processor converts terminal and application events into state changes.
- A stateful view registers its active scroll area and builds the content required for the current screen.
- Component and facade functions construct owned `Line<'static>` or `Span<'static>` values.
- The view processes hover and click metadata against the constructed lines.
- Ratatui widgets render the final lines, paragraphs, lists, tables, and scrollbars.

A view should not execute application side effects directly. It should store a `UiAction` in `AppState`, then let the state processor perform clipboard operations, file opening, navigation, or other actions.

## Function Schemes

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

## Action Dispatch Lifecycle

Views do not execute `UiAction` values directly.

The dispatch sequence is:

- A view detects a click or key shortcut.
- The view calls `state.set_action(action)`.
- `AppState` triggers a redraw.
- The state processor reads and clones the pending action.
- The state processor performs the side effect.
- The action is cleared after handling.

Current actions include:

- `ToClipboardCopy`, handled through the application clipboard and popup flow.
- `OpenFile`, handled through the automatic editor selection flow.
- `GoToTask`, handled by switching to the task tab and allowing the task view to select the target task.
- `WorkConfirm`, `WorkCancel`, `WorkRun`, and `WorkClose`, handled by forwarding execution events to the executor.

This keeps side effects centralized and allows components to remain testable and reusable.

## Current Trade-offs and Extension Points

The current architecture favors explicit control over abstraction.

Advantages include:

- Components construct exactly the span layout required by a view.
- Views choose the most appropriate scrolling strategy.
- Actions remain centralized and testable.
- Grouped zones support multiline content without changing model types.
- Styles and path detection are reusable across logs, pins, errors, input, output, and AI sections.

Current costs include:

- Hover detection and dispatch are repeated across several views.
- Scroll indicator calculations are written per view.
- Link-zone line offsets are manually maintained.
- Fixed-width components require narrow-terminal validation.
- Page scrolling is fixed rather than viewport-relative.
- A shared scroll identifier can intentionally or unintentionally share position across tabs.

Potential future abstractions should preserve the existing responsibilities:

- A shared scroll viewport helper could centralize clamping, visible ranges, and indicators.
- A shared link-zone dispatcher could centralize specificity selection and hover application.
- A logical line builder could own line offsets and separator accounting.
