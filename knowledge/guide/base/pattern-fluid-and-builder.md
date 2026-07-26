# Fluid API and Builder Patterns

Covers Rust conventions for fluid constructors and chainable methods on structs, and the Builder pattern for complex or fallible construction.

## When to Use

- **Fluid API on struct** – when construction is infallible and the struct is a simple value object or an options container.
- **Builder** – when construction requires validation, may be fallible, or the target has a complex inner architecture (e.g., using `Arc`). Builders are often infallible. Access via `Type::builder()`.

## Fluid API (Fluid Constructors & Fluid Chainable) on Struct

The fluid API — fluid constructors and fluid chainable methods — applies directly on types when construction is straightforward.

The main pattern is as follows:


```rust
#[derive(Default)]
pub struct SomeType {
    name: String,
    max_token: Option<u16>,
    messages: Vec<Message>,
}

/// Constructors
impl SomeType {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn from_max_token(max_token: u16) -> Self {
        Self {
            max_token: Some(max_token),
            ..Default::default()
        }
    }
}

/// Chainable setters
impl SomeType {
    pub fn with_max_token(mut self, max_token: u16) -> Self {
        self.max_token = Some(max_token);
        self
    }

    pub fn append_user_message(mut self, msg: Message) -> Self {
        self.messages.push(msg);
        self
    }

    pub fn append_user_messages(mut self, msgs: impl IntoIterator<Item = Message>) -> Self {
        self.messages.extend(msgs);
        self
    }
}

// region:    --- Froms

impl From<&str> for SomeType {
    fn from(name: &str) -> Self {
        SomeType::new(name.to_owned())
    }
}

// endregion: --- Froms

```

### Fluid Constructors

Place constructors in a dedicated `impl` block. Derive `Default` if there is a sensible default, and use `new()` for the primary constructor, `from_...()` for secondary ones. Accept `impl Into<T>` where appropriate.


- Use `from_...` constructors for secondary natural constructors. Keep the `From` trait implementations separate.
- Use `new()` with no arguments only when there is a single obvious constructor; otherwise prefer `Default` and `from_...`.

### Fluid Chainable Methods

Add `with_` and `append_` methods in a separate `impl` block. Consume `self` (not `&mut self`) for clean chaining.

- Name setter-like methods `with_...`.
- Name collection-adding methods `append_...` (singular for single item, `append_...s` for plural/`IntoIterator`).
- The consuming pattern avoids borrowing issues and composes cleanly.

### From Implementations

Standard `From` trait conversions go in separate `impl` blocks grouped together in one `Froms` code region, typically after the chainable block.

## Builder Pattern

Use a separate `Builder` struct when the target has a complex inner architecture (e.g., internal `Arc`) or when construction may be fallible (e.g., validation). The builder's setter methods follow the same fluid, consuming pattern (`with_`/`append_`). The `build()` method produces the final value — `Result<T, E>` for fallible builders, or `T` for infallible ones. Access the builder via `Type::builder()`.

```rust
pub struct Client {
    inner: Arc<ClientInner>,
}

struct ClientInner {
    endpoint: String,
    max_retries: u32,
}

impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }
}
```

The same fluid API pattern is applied to the builder, typically in a separate module or file (e.g., `client_builder.rs`):

```rust
#[derive(Default)]
pub struct ClientBuilder {
    endpoint: Option<String>,
    max_retries: Option<u32>,
}

/// Constructors & Chainables
... same pattern for ClientBuilder ...

```

