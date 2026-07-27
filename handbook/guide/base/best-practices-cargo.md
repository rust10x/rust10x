## Cargo.toml Best Practices

These are the best practices for cargo.toml and dependency management.

### Cargo.toml

Here is an example of a `Cargo.toml`:

````toml
[package]
name = "package-name"
version = "0.1.0"
edition = "2024"

[lib]
doctest = false

[lints.rust]
unsafe_code = "forbid"
# unused = { level = "allow", priority = -1 } # For exploratory dev.

[dependencies]
# -- Async
tokio = { version = "1", features = ["full"] }
# -- Json
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_with = { version = "3", features = ["macros"] }
# -- Others
derive_more = {version = "2", features = ["from", "display"] }
````

- In the `dependencies` toml section, do not add empty line above or below `# -- ...` comment lines. 
- Add the `[lib]` only if the crate is lib. 
- When lib, add the `doctest = false` under the `[lib]`
- By default, we will have the `lints.rust` section above, with the commented `unused` option that the user will toggle on and off during dev.
- The dependencies are organized by sections and should include those sections only if asked (except the `Others` section that can be added when starting).
- When starting, make sure to add the `# -- Others` if `derive_more` is present.
- The convention is to split the dependencies into sections with `# -- section_name`, and these are the very basic sections. But do not add empty line before or after this comment line. 
- Except if explicitly asked but, only add the `# -- Others` with `derive_more` by default, and then the section needed. 

Some popular dependencies sections
- `# -- Encoding` for all encryption, encoding, hash 


## resolver

When resolver needs to be specified use `resolver = "3"`
