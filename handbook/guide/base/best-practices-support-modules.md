## Support / Utils Module Best Practices

In Rust10x, the best practice for crate or sub-module utilities is the following:

Typically, when generic utilities don't fit into a clean sub-module name, we use the `support` module namespace to compartmentalize them for the appropriate module tree.

We use `support.rs` (or `support/mod.rs`), depending on how large they are.

For example:

````rust
// region: --- Modules

mod support;

//.. more modules

// endregion: --- Modules
````

Then, the sub-modules will use that support.

Note that they are designed to NOT BE public and only at the level of the modules for which they are intended.

So, the types and functions in support will be `pub ...`, but since it is defined as `mod support;`, it will only be accessible to this module and its sub-modules by design.

When support is needed across all of the sub-modules of a crate, it might be at the root.

Otherwise, some sub-modules might need `support` as well, and they will repeat this pattern.