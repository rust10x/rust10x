# Rust Edition 2024

## When to use this file

Use this file for guidance on Rust Edition 2024 specific features and modern practices.

## Overview

If no `Cargo.toml` or no Edition 2024 is specified, assume Edition 2024.

Here are some important new guidelines to follow when writing Rust code with modern best practices.

These are features of Rust Edition 2024 and modern Rust that should be followed.

Make sure to use them when appropriate.

## Future and IntoFuture

- The `Future` and `IntoFuture` traits are now part of the prelude.
- `IntoIterator` for `Box<[T]>`
    - Boxed slices implement `IntoIterator` in all editions.
    - Calls to `IntoIterator::into_iter` are hidden in editions prior to 2024 when using method-call syntax (i.e., `boxed_slice.into_iter()`). So, `boxed_slice.into_iter()` still resolves to `(&(*boxed_slice)).into_iter()` as it has before.
    - `boxed_slice.into_iter()` now calls `IntoIterator::into_iter` in Rust 2024.
- Cargo: Rust-version aware resolver
    - `edition = "2024"` implies `resolver = "3"` in Cargo.toml which enables a Rust-version aware dependency resolver.
- Cargo: Table/key consistency - now all with a `-` rather than `_` (e.g., `default-features`)
- Reject unused inherited default-features
    - default-features = false is no longer allowed in an inherited workspace dependency if the workspace dependency specifies default-features = true (or does not specify default-features).
- Async closures (see below)


## Hiding trait implementations from diagnostics

You can now use `#[diagnostic::do_not_recommend]` to suppress confusing trait suggestion messages.

More info: [RFC 2397](https://rust-lang.github.io/rfcs/2397-do-not-recommend.html), [Reference](https://doc.rust-lang.org/reference/attributes/diagnostics.html#the-diagnosticdo_not_recommend-attribute)



