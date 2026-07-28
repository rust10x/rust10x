# Rust Code Structure Best Practices

## When To Use

Always include this guide, together with `best-practices-general.md`, whenever writing, reviewing, or updating Rust code.

Use it to determine how source files, module files, utility modules, trait implementations, private support code, and tests should be organized.

## Source File Organization

Organize a Rust source file in the following general order:

1. Primary public types.
2. Supporting public types.
3. Public functions.
4. Public inherent implementations, grouped by purpose when useful.
5. Trait implementations.
6. Private support functions, implementations, and types.
7. Unit tests.

Keep the main type or types near the top of the file. Supporting types should follow the primary types they support.

When a file contains many types, they can be grouped in a `Types` region:

```rust
// region:    --- Types

// endregion: --- Types
```

Do not create regions merely to label every implementation block. Use regions for meaningful, larger sections of a file.

## Implementation Blocks

Organize inherent implementation blocks by purpose when doing so makes the public API easier to scan. Useful categories can include:

- Constructors and secondary constructors.
- Consuming chainable methods such as `with_...`, `append_...`, and `extend_...`.
- Property setters such as `set_...`.
- Accessors.

Separate trait implementations, such as `From`, `Default`, and iterator implementations, from inherent implementations.

Group `From` implementations in a `Froms` region:

```rust
// region:    --- Froms

impl From<Source> for Target {
	fn from(value: Source) -> Self {
		Target::from_source(value)
	}
}

// endregion: --- Froms
```

## Private Support Code

Place private functions, implementations, and types that are specific to one source file in a `Support` region after the public API and trait implementations:

```rust
// region:    --- Support

fn normalize_value(value: &str) -> String {
	value.trim().to_lowercase()
}

// endregion: --- Support
```

When utilities are shared by several files in the same module tree, place them in `support.rs` or `support/mod.rs` instead of duplicating them or keeping them in an unrelated source file.

## Tests

Place inline unit tests last, after the `Support` region, and wrap them in a `Tests` region:

```rust
// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
}

// endregion: --- Tests
```

When unit tests become large, move them to a sibling `_tests.rs` file or an appropriate file under `src/_tests`, while retaining the `Tests` region in the source file:

```rust
// region:    --- Tests

#[cfg(test)]
#[path = "applier_tests.rs"]
mod tests;

// endregion: --- Tests
```

Follow `best-practices-test.md` for test naming, setup, execution, checks, result types, and test support utilities.

## Module Files

Use `mod.rs` primarily for module declarations and intentional reexports. Put those declarations and reexports in a `Modules` region after any module-level documentation:

```rust
//! Module-level documentation.

// region:    --- Modules

mod event_base;
mod support;

pub use event_base::*;

// endregion: --- Modules
```

Implementation details should normally live in dedicated source files rather than in `mod.rs`.

## Support Modules

Use `support.rs` or `support/mod.rs` for generic utilities that do not fit a clearer module name and are intended only for a particular module tree.

Declare a support module privately:

```rust
mod support;
```

Items inside the support module can use `pub` or a narrower visibility where needed. Because the module itself is private, its public items remain limited by the visibility of the module path.

Place a support module at the crate root only when it is needed across the crate. Otherwise, place it at the narrowest module level shared by its consumers.

Call support items through the appropriate Rust module path for the caller. Do not require a fixed path such as `super::super::...`, since the correct path depends on the caller's location and the module's visibility.

## Common Modules

Use `common.rs` or `common/mod.rs` when utility types or functions are intentionally part of a broader API and should be reexported beyond their immediate parent module.

Declare and reexport common items explicitly:

```rust
mod common;

pub use common::*;
```

Choose between `support` and `common` based on intended visibility:

- Use `support` for implementation utilities limited to a module tree.
- Use `common` for shared utilities intentionally exposed through the containing module's API.

Prefer a more specific module name over either `support` or `common` when the utilities form a clear domain concept.

## Related Guides

Use the related guides for their focused requirements:

- `best-practices-general.md` for idiomatic Rust APIs, error handling, language features, and general implementation practices.
- `best-practices-comments.md` for exact region and section-marker formatting.
- `best-practices-test.md` for unit tests, integration tests, and test support.
