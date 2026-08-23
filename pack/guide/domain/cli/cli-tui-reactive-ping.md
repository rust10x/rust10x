# Reactive Demand-Driven Ping Timer

## Purpose

This guide describes the reactive demand-driven ping timer pattern for Ratatui TUI applications. It explains how to drive continuous animations, frame stepping, elapsed time counters, and auto-dismissing notifications with zero CPU overhead when the application is idle.

## Problem Context

In event-driven terminal user interfaces, rendering is typically triggered by incoming user events, such as key presses, mouse interactions, or background worker events.

When background processes are active (such as long-running jobs, task executions, or pack installations), the UI needs continuous updates to:

- Step spinner frames and pulse status indicators.
- Increment running duration timers smoothly.
- Expire and auto-dismiss timed notification popups.

An unconditional continuous loop (for example, ticking every 100ms regardless of state) burns CPU cycles and drains battery power even when the application is completely idle.

## Architecture and Core Mechanics

The reactive ping pattern establishes a self-sustaining feedback loop that activates only when demand exists.

```text
+-------------------------------------------------------------+
|                      Main UI Loop                           |
|                                                             |
| 1. Handle AppEvent::Tick(ts)                                |
| 2. Update state.time = now_micro()                          |
| 3. Render Terminal View                                     |
| 4. Evaluate `app_state.should_be_pinged()`                  |
|    - If true  --> Send now_micro() to PingTimerTx           |
|    - If false --> Do nothing (worker idles)                 |
+------------------------------+------------------------------+
                               |
                   ts (i64)    |    AppEvent::Tick(ts)
                               v    ^
+-----------------------------------+-------------------------+
|                  Ping Timer Worker Task                     |
|                                                             |
| - Waits for incoming ping timestamp                         |
| - Debounces window (e.g., 100ms) with tokio::time::sleep    |
| - Dispatches AppEvent::Tick back to AppTx on timer fire     |
+-------------------------------------------------------------+
```

### 1. Demand Detection (`should_be_pinged`)

At the conclusion of each event processing cycle, the application queries a declarative state predicate:

```rust
impl AppState {
    pub fn should_be_pinged(&self) -> bool {
        self.running_tick_count().is_some()
            || self.popup().is_some_and(|p| p.is_timed())
            || matches!(self.stage(), AppStage::Installing | AppStage::Installed)
    }
}
```

The predicate returns `true` if any visual element requires continuous re-rendering.

### 2. Ping Timer Worker Service

The ping timer runs in an isolated asynchronous worker task. It receives ping requests over a dedicated channel (`PingTimerTx`) and delivers `AppEvent::Tick` events back to the primary application channel (`AppTx`).

```rust
use derive_more::{Deref, From};
use std::pin::Pin;
use tokio::task::JoinHandle;
use tokio::time::{Duration, Sleep};

#[derive(Clone, From, Deref)]
pub struct PingTimerTx(Tx<i64>);

pub fn start_ping_timer(app_tx: AppTx) -> Result<PingTimerTx> {
    let (tx, rx) = new_channel::<i64>("ping_timer");
    let _handle = run_ping_timer(rx, app_tx);
    Ok(PingTimerTx::from(tx))
}

fn run_ping_timer(rx: Rx<i64>, app_tx: AppTx) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut pending_ts: Option<i64> = None;
        let mut sleep_fut: Option<Pin<Box<Sleep>>> = None;

        loop {
            if let Some(sleep) = sleep_fut.as_mut() {
                tokio::select! {
                    _ = sleep.as_mut() => {
                        if let Some(ts) = pending_ts.take() {
                            let _ = app_tx.send(AppEvent::Tick(ts)).await;
                        }
                        sleep_fut = None;
                    }
                    msg = rx.recv() => {
                        match msg {
                            Ok(ts) => {
                                pending_ts = Some(ts);
                            }
                            Err(_) => break,
                        }
                    }
                }
            } else {
                match rx.recv().await {
                    Ok(ts) => {
                        pending_ts = Some(ts);
                        sleep_fut = Some(Box::pin(tokio::time::sleep(Duration::from_millis(100))));
                    }
                    Err(_) => break,
                }
            }
        }
    })
}
```

### 3. Worker Debounce Coalescing

When multiple events or rapid state transitions send multiple pings while a timer sleep future is active:

- The worker updates `pending_ts = Some(ts)` with the freshest timestamp.
- The existing sleep timer continues running without being reset.
- When the timer expires, exactly one `AppEvent::Tick` is dispatched with the latest timestamp.

### 4. Self-Sustaining Feedback Loop

In the UI main loop (`tui_loop.rs`):

- When `AppEvent::Tick(ts)` arrives, the loop updates `app_state.core.time` and redraws the screen.
- State processing runs, updating animation offsets and checking popup expiration.
- At the end of the cycle, `app_state.should_be_pinged()` is re-evaluated.
- If demand remains active, `ping_tx.send(now_micro()).await` is sent, triggering the next timer cycle.
- When all active tasks complete and popups close, `should_be_pinged()` returns `false`. No ping is sent, and the worker sleeps until new external activity occurs.

## Animation Math and Frame Stepping

Animation frame indices and flag toggles are derived mathematically from elapsed microseconds rather than mutable tick counters.

### Frame Tick Count Calculation

```rust
pub fn tick_count(duration_micro: i64, interval_sec: f64) -> i64 {
    let interval_micro = (interval_sec * 1_000_000.0) as i64;
    if interval_micro <= 0 {
        return 0;
    }
    duration_micro / interval_micro
}
```

### Spinner and Flag Derivation

In `AppState`:

```rust
impl AppState {
    pub fn running_tick_count(&self) -> Option<i64> {
        let running_start = self.core().running_tick_start?;
        let duration_micro = (self.core().time - running_start).max(0);
        let ticks = tick_count(duration_micro, 0.2); // 5 Hz step
        Some(ticks)
    }

    pub fn running_tick_flag(&self) -> Option<bool> {
        let ticks = self.running_tick_count()?;
        Some((ticks / 3) % 2 == 0) // Alternates cadence every 3 ticks
    }
}
```

Using elapsed time relative to start time ensures animation frames remain in sync even under variable frame rendering rates.
