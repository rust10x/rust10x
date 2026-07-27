# Rust10X test best practices and conventions


## When To Use

Use this file when writing or updating Rust test code. 


## Unit Test


For Rust unit tests, here is a good template to follow:

- Let's assume the file is `src/support/text.rs`
    - Or one that gets flattened to this. For example, if `src/support/text/mod.rs` does a `pub use text_common::*` for the file `src/support/text/text_common.rs`
- The function to test would be `crate::support::replace_markers(...)`
- When the test is inside a binary or library code file, with `mod tests {`, follow this layout:

```rust
// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

  use super::*;

  #[test]
	fn test_support_text_replace_markers_simple() -> Result<()>{
        // -- Setup & Fixtures
        // ... here the code that preps/sets the context for the tests

        // -- Exec
        // ... here the code that executes the function to be tested

        // -- Check
        // ... here all of the checks/asserts
        // ... can be commented like `// check the blocks` and multiple lines below
    }

  #[test]
	fn test_support_text_replace_markers_with_filter() -> Result<()>{
        // ... same structure as above.
    }

    // region:    --- Support
    // ... support functions that might be used in above code. 
    // endregion: --- Support
}

// endregion: --- Tests

```

- For these types of test blocks, have `use super::*;` as shown above.
- Do not use `unwrap()`. Instead, use an `.ok_or("Should be ...")?`-style pattern when getting expected values (this works well with the boxed dyn error type).
- Make sure to have `// region:    --- Tests` at the top level. It should surround the `#[cfg(test)] mod test {...}`.
    - If there is no `#[cfg(test)]`, which means the file is a dedicated test file, there is no need to add `// region:    --- Tests`.

- Include clear section comments in every test function:
  - `// -- Setup & Fixtures`
    - Use this section to initialize the environment and set up any necessary data or context.
  - `// -- Exec`
    - Put the code that executes the function under test here for clarity.
  - `// -- Check`
    - Use this section to include assertions and verify that the expected outcome is met.
  - `// -- Exec & Check`
    - Use this section when exec and checks are in the `for` loop, and put it above the `for ...`.

- Ensure the tests are wrapped in a dedicated test module with region comments:
  
  - Use:
    - `// region:    --- Tests` at the beginning, and
    - `// endregion: --- Tests` at the end.
  - This helps with visual grouping and organization of the tests within any module.

- For tests that require helper functions:
  
  - Have a nested support region marked with:
    - `// region:    --- Support`, and
    - `// endregion: --- Support` for additional helper functions used only during tests.

- Always define a dedicated type alias for test results at the top of your tests:
  - For instance:
    - `type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;`
  - This ensures consistent error handling and improves test readability.

- The name of the test function has the following format:
- `test_[module_path_name]_[function_name]_[variant]()`
- So, for example, we would have a function name like `test_support_text_replace_markers_simple`
    - To test a function `replace_markers` in the module path `src/support/text`
    - `support_text` is the module path name (make sure to look at the mod.rs, because submodules can be flattened out).
    - `replace_markers` is the function being tested.
    - `simple` is the first variant and the first one to implement.
    - `with_filter` is another variant to show that sometimes we want to test different things.
- No need to repeat the crate or library name in the test function name.
    - For example, if the crate or library is named `simple_fs`, do not use a name like `test_simple_fs_support_text_...`. That would be silly.
    - Just use `test_support_text_...`.

- When you need to create temp data files, for each function, use paths like `tests-data/.tmp/test_function_name/`, where `test_function_name` is the full unit test function name. This way, all tests have their own directory.
- IMPORTANT: For cleanup, comment out the `..remove..` code so the user has to enable it manually. This ensures we do not remove files automatically.


### Split unit tests in its file

When the unit tests in a source file become huge, we can split them into their own file by either adding a `_tests.rs` suffix, or putting them in `src/_tests/tests_module_flatten_to_leaf_file.rs`

For example, on the former, if the `applier.rs` test become big, and if user ask for it, we can split it by adding this 

```rs
// region:    --- Tests

#[cfg(test)]
#[path = "applier_tests.rs"]
mod tests;

// endregion: --- Tests
```


## For integration tests

- Follow the best practices above.
- The file does not need to include the comment section `// region:    --- Tests`.
- All tests will be in files like `tests/tests_.....rs`, using the same suffix pattern mentioned above.
- Each integration test will still use `type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.`

## test_support

Unit tests and integration tests can have test support functions.

- For unit tests, they will be under `src/_test_support/mod.rs` (exported as `test_support`).
- For integration tests, they will be under `tests/test_support/mod.rs`.
- The `mod.rs` will look something like:

```rust
//! Some support utilities for the tests
//! Note: Must be imported in each test file

#![allow(unused)] // For test support

// region:    --- Modules

mod asserts;
mod helpers;
mod seeders;

pub use asserts::*; // could be assert utilities
pub use helpers::*; // could be some test dir/file create/delete utilities 
pub use seeders::*; // could be some data creation data. 

type TestResult<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

// endregion: --- Modules
```

`TestResult` will use the same pattern as the other test result type, but it will have a specific alias so all tests can use the same type alias.

In `helpers.rs`, one good function to have is `let test_dir = test_support::new_out_dir_path(prefix)?`.

This will create a path like `tests/.out/prefix_unixtime_ms`, which will be unique enough and allow the test to create files as needed.

Then you can use `test_support::delete_out_dir(test_dir)`, which will first verify that `test_dir` is below the current directory and contains `tests/.out` in the path, making it safer to remove.
