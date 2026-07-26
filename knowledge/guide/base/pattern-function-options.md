# Function Options Pattern

When a function accepts optional configuration, use `impl Into<Options>` together with a `From<Option<Options>>` implementation.

This lets callers pass `None`, `Some(opts)`, or the options struct directly, without importing the struct for the `None` case.

## When to Use

Use this guide when you have a function that takes an `Options` struct argument (i.e., a struct holding optional configuration parameters). The pattern presented here simplifies the caller API by allowing `None` defaults.

## Example

```rust
#[derive(Debug)]
pub struct ProcessOptions {
    pub verbose: bool,
}

impl Default for ProcessOptions {
    fn default() -> Self { Self { verbose: false } }
}

impl From<Option<ProcessOptions>> for ProcessOptions {
    fn from(o: Option<ProcessOptions>) -> Self { o.unwrap_or_default() }
}

fn process(txt: &str, opts: impl Into<ProcessOptions>) {
    let opts = opts.into();
    // ...
}

// Usage:
process("data", None);
process("data", ProcessOptions { verbose: true });
process("data", Some(ProcessOptions { verbose: true }));
```

## When to Use This Pattern

- The options struct has a natural `Default`.
- The function should accept either default configuration or a configured instance.
- Simplifies the API for callers who just want the defaults—no import or `::default()` needed at the call site.

## Notes

- For more complex, fallible, or multi-step construction, prefer a separate builder pattern (e.g., `ProcessOptionsBuilder` with `.build() -> Result<ProcessOptions>`).
- The `impl Into<ProcessOptions>` parameter is consumed by value. If the options struct is large, consider passing a reference or using a builder that produces a lightweight config.
