## Error Best Practices

This outlines the error best practices for Rust10x.

Rust10x does not utilize `thiserror` or `anyhow`, but rather a traditional Rust error pattern.

The goal is to have a similar and progressive approach between tests/examples and application/lib code.

### For Tests and Examples

For tests and examples, use the type alias `pub type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;`

- For test modules, we will have the type alias so that we can simply use `Result<T>`. Make sure to fully qualify `core::result::Result` as the test might `use super::*` which will introduce another application `Result<T>`.

- For example files, when there is only one `main` function, we can define `fn main() -> Result<(), Box<dyn std::error::Error>> { ... }`

### For App & Lib Code

Typically, in the `lib.rs` or `main.rs` we will have the following:


````rust
// file: src/main.rs
// region:    --- Modules

mod error;

pub use error::{Error, Result};

// endregion: --- Modules

fn main() -> Result<()> {
	println!("Hello, world!");

	Ok(())
}
````

- The region `// region: --- Modules` is a Rust10x best practice at the top of each `mod.rs`, `lib.rs`, `main.rs` to group all `mod`, `pub mod`, and `use` statements.
- In other Rust files, we do not need to have this region.
- The error pattern is to have a `mod error` which will define the `Error` type.
- For the main code, the best practice is to use `mod error` and then re-export (flatten) `pub use error::{Error, Result};`

### The `error.rs`

The `error.rs` has the following format:

```rust
use derive_more::{Display, From};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Display, From)]
#[display("{self:?}")]
pub enum Error {
	#[from(String, &String, &str)]
	Custom(String),

	// -- Externals
	#[from]
	Io(std::io::Error), // as example
}

// region:    --- Custom

impl Error {
	pub fn custom(val: impl Into<String>) -> Self {
		Self::Custom(val.into())
	}
	
	pub fn custom_from_err(err: impl std::error::Error) -> Self {
		Self::Custom(err.to_string())
	}


}

// endregion: --- Custom

// region:    --- Error Boilerplate

impl std::error::Error for Error {}

// endregion: --- Error Boilerplate

```

- We have the `Custom` region and variant for early flexibility.
- By including `#[from(String, &String, &str)]` on the `Custom` variant, we enable seamless conversion from string literals and formatted strings. For example, this allows the use of `.ok_or("error message")?`, `ok_or_else...`, use the `.into()` on the static string or format!, rather than use `Error::custom...`. This way, simpler and more idiomatic. `Error::custom` should be use when a simple closure can be pass as function.
- Below `// -- Externals` are the external types, usually passed through.
- Above will be for the application specific errors.
- `Custom` variant is always on top.
- Then, we might have comment sections like `// -- Some Sub Module` to categorize the variants.
