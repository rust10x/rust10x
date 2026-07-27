# Rust Documentation Strategy

## Documentation Layout

Keep source files concise and move module-level documentation to Markdown files.

```text
docs/
├── rustdoc/
│   ├── lib.md
│   ├── params/
│   │   ├── mod.md
│   │   └── schema.md
│   └── runtime/
│       └── mod.md
└── for-llm/
    ├── overview.md
    ├── architecture.md
    ├── concepts.md
    ├── project-map.md
    └── coding-conventions.md
```

### Purpose

- `docs/rustdoc/`: Human-facing documentation included into rustdoc.
- `docs/for-llm/`: Documentation optimized for AI agents and code generation.

The `for-llm` name explicitly means "documentation for LLMs", not "documentation about LLMs".


## Source Documentation

Keep only concise API documentation in Rust source:

```rust
/// Parameters accepted by a program.
pub trait Params {
	/// ...concise but complete doc...
	name: String
}
```

Attach larger documentation via `include_str!` at the very top of the files

src/lib.rs
```rust
#![doc = include_str!("../docs/rustdoc/lib.md")]

... rest of the code ..
```

src/params/schema.rs
```rust
#![doc = include_str!("../../docs/rustdoc/params/schema.md")]

... rest of the code ..
```

### Module Documentation Attribute Placement

Place the `#![doc = include_str!(...)]` attribute inside the module's own Rust source file, not as an outer attribute on the `mod` declaration in the parent module.

**Do this instead:**

src/elem.rs
```rust
#![doc = include_str!("../docs/rustdoc/elem.md")]
```

The inner attribute (`#!`) applies to the enclosing item (the module itself), which is the correct scope for module-level documentation.

### Function and Type Documentation

When a module's primary API is a single function or type (especially when the module merely flattens re-exports), attach the documentation directly to that item rather than the module. This keeps the documentation scoped correctly and avoids an empty module doc.

Use an outer attribute on the item:

```rust
#[doc = include_str!("../docs/rustdoc/foo/bar/fn_name.md")]
pub fn my_fn() { ... }
```

Follow the same directory mapping, using a file named after the item (e.g., `docs/rustdoc/crate/module/fn_name.md` for a function named `fn_name` in `crate::module`).


## Directory Mapping

Mirror the source tree inside `docs/rustdoc`.

```text
src/foo/bar.rs
docs/rustdoc/foo/bar.md

src/foo/mod.rs
docs/rustdoc/foo/mod.md

src/lib.rs
docs/rustdoc/lib.md
```

Mirroring scales better and keeps paths predictable.

## Guiding Principles

1. Keep Rust source focused on code.
2. Keep only short API docs in source.
3. Store module guides in Markdown.
4. Mirror the source tree under `docs/rustdoc`.
5. Keep AI-oriented documentation under `docs/for-llm`.
6. Re-export derive macros through a `derive` module for ergonomics.
7. Prefer a separate `-derive` crate while only derive macros exist.
8. For silent modules (only re-exports), attach documentation directly to the exported item rather than the module.
