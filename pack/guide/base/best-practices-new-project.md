# New Rust Project Best Practices

## When to use this file

Use this file when starting new Rust projects (binary or library) or experimental (`xp`) learning projects, covering project initialization, naming conventions, and initial file structure.

## New project common best practices


- When users start a new project without specifying "xp" or "library," assume it is a binary project.

- And `xp` project is for eperimental project typically used to learn a pattern or library (See below)

- Follow the cargo-best-practices.md when creating and managing the `Cargo.toml`


## xp project Best Practices


- Sometimes the user may create an `xp-...`-like project, where `xp` stands for experiment or exploration.

- For example, if the goal of the `xp-...` project is to learn about `blake3`, the name is `xp-blake3`.

- So, this will be a lib project with a `src/lib.rs` file (empty to start with),

- Start with an empty `src/lib.rs` unless the user asks specifically.

- with `examples/c01-simple.rs` 

- if asked to be async, add the tokio cargo group, and make the example tokio async. 

- When doing async do not do `use tokio::main` but `#[tokio::main] async fn main...`

- The content of the `main` example function should be

```rust
println!("Hello World");

Ok(())
```
