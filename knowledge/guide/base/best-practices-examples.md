# Example Best Practices

## When to use this file

Use this file when creating or updating Rust example files in the `examples/` directory, including naming conventions (c01...), standard `main` signatures, and simplified error handling strategies for examples.

## Example important rules

- When creating example files, a good naming convention is to use chapters, guiding the user from simple to more complex examples.

- The convention is to have `examples/c01-simple.rs` for the simplest case.

- Then, for each topic, use `examples/c02-some-functionality.rs` for a given functionality.

- The goal is for each of these examples to focus on one aspect of functionality from the main crate or for learning about another crate (in the case of an `xp-project`).

The main signature will be:

- `fn main() -> Result<(), Box<dyn std::error::Error>>` (for example files, do not `use std::error::Error`; just use it this way)

- In most cases, we don't need a type alias for these, since it's only one file, and that will allow exporting the crate's `Result/Error` if needed.
