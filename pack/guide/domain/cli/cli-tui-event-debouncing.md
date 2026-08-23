# TUI Front Event Debouncing and Coalescing

## Purpose

This guide describes the front event draining and coalescing pattern for Ratatui TUI event loops. It explains how to prevent UI lag, event queue backup, and dropped frames when processing bursty event streams from background workers, file watchers, or rapid mouse movement.

## Problem Context

A terminal user interface typically listens to multiple event sources multiplexed into a single `AppRx` receiver channel:

- Terminal keyboard and mouse events.
- Background worker status and execution updates.
- IPC or SQLite data change notifications.
- Timer tick signals.

Under high load (such as streaming task outputs or fast mouse scrolling), events may arrive significantly faster than the terminal can render frames. Directly rendering after every single received event leads to:

- Terminal draw latency and input stutter.
- Unnecessary database queries or state rebuilds for intermediate states.
- Event channel buffer exhaustion.

## Architecture and Core Mechanics

The front event debouncer uses a non-blocking channel drain (`try_recv`) immediately following an initial asynchronous wait (`recv().await`).

```text
               +----------------------------------------+
               |           AppRx Event Channel          |
               +----------------------------------------+
                                   |
                  1. Async wait    | `recv().await`
                                   v
               +----------------------------------------+
               |        First Received Event            |
               +----------------------------------------+
                                   |
                  2. Non-blocking  | `while let Ok(Some(evt))`
                     drain loop    |  = app_rx.try_recv()
                                   v
               +----------------------------------------+
               |           Debouncer Collector          |
               |                                        |
               | - UI/Action/Keys  --> Retained (FIFO)  |
               | - Model/Task Data --> Collapsed by ID  |
               | - Redraw Signals  --> Collapsed to one |
               | - Timer Ticks     --> Latest only      |
               +----------------------------------------+
                                   |
                  3. Ordered batch | `into_events()`
                                   v
               +----------------------------------------+
               |       Sequential State Updates         |
               |       Followed by Terminal Draw        |
               +----------------------------------------+
```

## Implementation

### 1. The Debouncer Collector

The collector sorts and coalesces events by domain semantics:

```rust
use std::collections::HashMap;

struct Debouncer {
    last_redraw_event: Option<AppEvent>,
    ui_events: Vec<AppEvent>,
    event_by_run_id: HashMap<Id, AppEvent>,
    tick_event: Option<AppEvent>,
}

impl Debouncer {
    fn new(first_event: AppEvent) -> Self {
        let mut debouncer = Self {
            last_redraw_event: None,
            ui_events: Vec::new(),
            event_by_run_id: HashMap::new(),
            tick_event: None,
        };
        debouncer.process(first_event);
        debouncer
    }

    fn process(&mut self, app_event: AppEvent) {
        match app_event {
            AppEvent::DoRedraw => {
                self.last_redraw_event = Some(AppEvent::DoRedraw);
            }
            AppEvent::Term(event) => {
                self.ui_events.push(AppEvent::Term(event));
            }
            AppEvent::Action(action_event) => {
                self.ui_events.push(AppEvent::Action(action_event));
            }
            AppEvent::Model(model_event) | AppEvent::Hub(HubEvent::Model(model_event)) => {
                let for_run_id = match &model_event.entity {
                    EntityType::Task if let Some(run_id) = model_event.rel_ids.run_id => Some(run_id),
                    EntityType::Run if let Some(run_id) = model_event.id => Some(run_id),
                    _ => None,
                };

                let app_event = AppEvent::Model(model_event);
                if let Some(run_id) = for_run_id {
                    // Retain only the most recent event for this specific run
                    self.event_by_run_id.insert(run_id, app_event);
                } else {
                    self.last_redraw_event = Some(app_event);
                }
            }
            AppEvent::Hub(hub_event) => {
                self.last_redraw_event = Some(AppEvent::Hub(hub_event));
            }
            AppEvent::Tick(tick) => {
                // Collapse multiple pending ticks into the newest tick timestamp
                self.tick_event = Some(AppEvent::Tick(tick));
            }
        }
    }

    fn into_events(self) -> Vec<AppEvent> {
        let mut events = self.ui_events;
        events.extend(self.event_by_run_id.into_values());
        if let Some(last_redraw_event) = self.last_redraw_event {
            events.push(last_redraw_event);
        }
        if events.is_empty() && let Some(tick) = self.tick_event {
            events.push(tick);
        }
        events
    }
}
```

### 2. Draining Channel Loop

```rust
fn debounce_events(app_rx: AppRx, first_event: AppEvent) -> (AppRx, Vec<AppEvent>) {
    let mut debouncer = Debouncer::new(first_event);
    loop {
        match app_rx.try_recv() {
            Ok(Some(app_event)) => {
                debouncer.process(app_event);
            }
            Ok(None) => break,
            Err(_) => break,
        }
    }

    let events = debouncer.into_events();
    (app_rx, events)
}
```

### 3. Main Event Loop Integration

In `tui_loop.rs`:

```rust
let app_event = match app_rx.recv().await {
    Ok(app_event) => app_event,
    Err(err) => {
        error!("Channel closed: {err}");
        break;
    }
};

let (new_app_rx, events) = debounce_events(app_rx, app_event);
app_rx = new_app_rx;

for app_event in events {
    // Process terminal mouse state
    if let AppEvent::Term(TermEvent::Mouse(mouse_event)) = &app_event {
        app_state.set_mouse_event(mouse_event);
    }

    // Render current frame
    let _ = terminal_draw(&mut terminal, &mut app_state);

    // Dispatch handler
    let _ = handle_app_event(
        &mut terminal,
        app_state.mm(),
        &executor_tx,
        &app_tx,
        &exit_tx,
        &app_event,
    ).await;

    // Advance state machine
    process_app_state(&mut app_state, process_opts);
}
```

## Key Benefits

- **Zero Lag Under Load**: Bursting logs or mouse gestures are condensed into atomic update passes.
- **Strict Input Ordering**: User keystrokes and navigation commands are never dropped or reordered.
- **Entity State Deduplication**: Multiple rapid task updates for the same parent run trigger only one task reload.
- **Reduced Rendering Overhead**: Unnecessary intermediate redraws are collapsed into a single final frame.
