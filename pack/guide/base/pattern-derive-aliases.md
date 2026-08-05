## Derive Aliases Best Practices

If the user explicitly asks to use derive aliases, use the `macro_rules_attribute` crate and its `derive_alias!` declarative macro.

The rules below define how to wire it into a project.

### Dependency in Cargo.toml

Add `macro_rules_attribute = "0.2"` to Cargo.toml under the `# -- Others` section, right below `derive_more` if present.

### Module structure best practices

Place crate-wide derive aliases in `src/derive_aliases.rs`. Keep declarative macros in the top-level `src/macros/` module, where they are accessed through `crate::macros::...`.

For example:

```rust
// src/derive_aliases.rs

use macro_rules_attribute::derive_alias;

derive_alias! {
	// Crate-wide derive aliases.
	#[derive(Cmp!)] = #[derive(PartialEq, Eq, PartialOrd, Ord)];
}
```

```rust
// src/macros/mod.rs

// region:    --- Modules

mod from_optional;

pub use from_optional::FromOptional;

// endregion: --- Modules
```

```rust
// src/lib.rs or src/main.rs

mod derive_aliases;
use derive_aliases::*;

pub mod macros;
```

This structure keeps derive aliases available at the crate root and makes declarative macros available through an intentional `crate::macros` namespace.

If derive aliases are only needed for a specific layer, place `derive_aliases.rs` at that layer root instead. Import them in that layer's `mod.rs` and make them available to submodules via `use derive_aliases::*;`.

For example, `src/model/mod.rs` can contain:

```rust
mod derive_aliases;

use derive_aliases::*;
```

### How to define a `derive_aliases.rs`

Here is a concise starting point for the root `derive_aliases.rs`

```rust
use macro_rules_attribute::derive_alias;

derive_alias! {
	// Basic compare (no float)
	#[derive(Cmp!)] = #[derive(PartialEq, Eq, PartialOrd, Ord)];

	// Basic hash (e.g., for hash keys)
	#[derive(Hash!)] = #[derive(PartialEq, Eq, Hash)];

    // For enum as str
	#[derive(EnumAsStr!)] = #[derive(strum::IntoStaticStr, strum::AsRefStr)];
}
```

The first two (Cmp, Hash) can be added by default, and the one about EnumAsStr only if `strum` is part of the Cargo.toml.

Some submodules might have their own `derive_aliases.rs`. While the goal is not to create one for each layer, for the key module layers that might be a good idea if the user asks. Some of those aliases will probably be compositions of the crate-wide aliases.

For example, for a `src/model/derive_aliases.rs` we could have 

```rust
use macro_rules_attribute::derive_alias;

derive_alias! {
	#[derive(ScalarStruct!)] = #[derive(
		crate::Cmp!,
		Clone,
		Copy,
		Hash,
		derive_more::From,
		derive_more::Into,
		derive_more::Display,
		derive_more::Deref,
		modql::SqliteFromValue,
		modql::SqliteToValue
	)];
}
```

In this example, this is an alias for a simple tuple struct that wraps a primitive type. Adapt the list based on what the user wants. 

Note that we can use the `crate::Cmp!` that was defined, and even in the same `derive_alias!` we can use aliases defined earlier in the block. 

Also, note that when in a `derive_alias!...` we use a fully qualified alias name like `crate::Cmp!`

### Usage of the derive aliases

Use `macro_rules_attribute`'s `derive` proc macro. Keep it namespaced (short), for example `mra`.

For example: 

```rust
use macro_rules_attribute as mra; 
use crate::model::ScalarStruct;
use crate::Cmp;

#[mra::derive(Debug, ScalarStruct!)]
pub struct Id(i64);

#[mra::derive(Cmp!)]
pub struct OtherType(i32);
```

- Make sure that when we use `mra::derive` the aliases are in scope (for example, `use crate::model::ScalarStruct;`), so that `mra::derive` can stay on one line by default. Follow the existing layout if it is already multiline.

- The module alias `mra` is to make it short and clear that when we use the derive, it is not the standard derive. 

- No need to add a comment to the alias to say what it expands to. 

For crate-wide aliases, prefer `src/derive_aliases.rs`. Use a layer-specific `derive_aliases.rs` only when the aliases are not intended for the entire crate.

### DO NOT derive overlapping aliases

If two aliases expand to any common derive traits, do not use them together. Choose the alias that covers the broader set, then add any missing derives explicitly.

For example, `Cmp!` and `Hash!` overlap, therefore they cannot be used in the same derive.
