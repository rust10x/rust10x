## Comments Best Practices

Here are the Rust10x comments best practices that build upon Rust comment best practices.

## When To Use

Use this guide whenever writing or updating comments, documentation comments, code regions, or code section markers in Rust code. Use it alongside the always-included `best-practices-general.md` and `best-practices-code-structure.md` guides.

Make sure to not add other comment styles for delimiting than those styles, except if the code already uses some patterns. 

### Comment Delimiters

Rust10x best practices include two main comment delimiters:

1. Code Region

```rust
// region:    --- Region Name

// endregion: --- Region Name
```
For larger code sections.

2. Code Section Marker

```rust
// -- Some Section Name
```

Typically used without a function body to section off lines of code. This does not have an end marker; it just indicates that from that point, the code block is about a specific section.

### Code Region

For large chunks of code, usually groups of types, functions, private/support code for a file, or a test section, use the code region formatted like this:

```rust
// region:    --- Region Name

// endregion: --- Region Name
```
- The `Region Name` is short, usually one or two words max.
- The spacing is important; it has to be exactly this number of spaces, for example, before and after the `---` to be fully aligned.
- These regions can be at the top level of a file or even within some function bodies if very large.
- Splitting methods into separate `impl` blocks by category does not require a code region. When a label is useful, use a concise documentation comment such as `/// Accessors` directly before the `impl` block.

Good candidates for code regions include:

- `From` implementations, grouped in one `Froms` region.
- Private support functions, implementations, or types specific to the current module, grouped in a `Support` region.
- Unit tests, grouped in a `Tests` region.

When these sections are present, each should use its corresponding code region. `Support` and `Tests` regions follow the same structural convention as the `Froms` region.

#### Do Not Add Regions Everywhere

Do not start every source file with a code region, and do not add a region just to have one. Add a region only when the file actually contains that category of code:

- Add a `Modules` region only in `main.rs`, `lib.rs`, and `mod.rs` files.
- Add a `Support` region only when the file has private support code.
- Add a `Froms` region only when the file has `From` implementations.
- Add a `Tests` region only when the file has inlined unit tests.

For `main.rs`, `lib.rs`, and every `mod.rs`, never add inline tests or a `Tests` region. Put tests in external sibling files named `main_tests.rs`, `lib_tests.rs`, or `mod_tests.rs`, and put only their conditional module declarations inside the `Modules` region.

When a file does hold several distinct sections, group each self-contained one in a region, such as a group of factory functions or one type together with its `impl` blocks. The module comment `//!` stays at the top, followed by the `use` declarations, and then the regions. A file with a single main type, or without distinct sections, does not need a region; the main type simply starts at the top.

The module-entry files are exceptions to that ordinary source-file ordering: their `Modules` region follows any module-level documentation. In `main.rs`, place the `use` declarations needed by `main` below the region. Keep `lib.rs` and `mod.rs` limited to module-level documentation and the module wiring/imports/reexports within that region.

#### Typical Code Regions

**In `main.rs`, `lib.rs`, and all `mod.rs` files only:**

- There will be a code region at the top (after the eventual module comments `//!`) named `Modules`.
- This will contain all of the `mod ..` imports and reexports like `pub use sub_module::*` when appropriate.
- Do not add this `Modules` region to any other file. An ordinary source file may declare an external test module inside its `Tests` region, but must not use a `Modules` region.

Keep `main.rs` to a minimal entry point that delegates application work to a submodule. Keep implementation details and types out of `lib.rs` and `mod.rs`, using appropriately named submodules, including `_impl` or `_types` when suitable. Intentional reexports may flatten the API. See the Module Files and Tests sections of `best-practices-code-structure.md` for the complete organization rules.

**For Tests Block in Source Code Files:**

- As described in the `test-best-practices`, when unit tests are inlined in a source file, the `#[cfg(test)] mod tests { ... }` block is wrapped in a `Tests` code region.

This inline pattern applies only to ordinary source files, never to `main.rs`, `lib.rs`, or `mod.rs`. Tests for those module levels belong in external test files as described above.

**Inlined Support Functions/Types:**

- Sometimes a source file will have some private functions specific to the file's logic. Before the eventual test code region, we have a `Support` code region with the private types/functions there.

**For `From` and similar blanket trait implementations:**

- After the type definitions and public function implementations, group all `impl From<...> for Type` blocks in a `Froms` code region.

```rust
// region:    --- Froms

impl<'a> From<&'a str> for HtmlContent<'a> {
	fn from(value: &'a str) -> Self {
		HtmlContent::Source(value)
	}
}

impl<'a> From<&'a HtmlParsed> for HtmlContent<'a> {
	fn from(value: &'a HtmlParsed) -> Self {
		HtmlContent::Parsed(value)
	}
}

// endregion: --- Froms
```


### Code Section Marker

In function bodies or types, code section markers `// -- some concise description` can be used to further split the code.

- These markers do not have a closing marker and are just intended to indicate: "Below is about this."
