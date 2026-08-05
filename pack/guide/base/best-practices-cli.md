# CLI Best Practices

This outlines the Rust10x CLI structure and best practices, ensuring a clean separation between the command-line interface and the domain logic.

## Overview

The goal is to keep the CLI layer thin and focused only on user interface concerns (parsing, argument validation, and output formatting), while the core functionality lives in "handlers" that are independent of the CLI.

## File Structure

The CLI logic is organized within the `src/cli/` directory:

- `src/cli/mod.rs`: Module definitions and re-exports.
- `src/cli/cmd.rs`: Clap structure definitions (Parser, Subcommands, Args).
- `src/cli/executor.rs`: The main entry point that routes commands.
- `src/cli/exec_<sub_command>.rs`: Topic-specific execution logic (bridges CLI to handlers).

The domain logic lives in:

- `src/handlers/`: Implementation of tasks, free of CLI artifacts (e.g., no `clap` types).

When user ask to do a very simple cli, with exec/all in main, then we have

- `src/cmd.rs`: Clap structure
- `src/main.rs`: the parse and exec logic

This way simpler to start and then, can be upgraded to the full structure later. 

## Best Practices

### 1. `src/cli/cmd.rs`

Define the CLI structure using `clap` with the `derive` feature.

- **CliCmd**: Root `Parser` struct. Usually has `Option<CliSubCmd>`.
- **CliSubCmd**: Top-level subcommands enum.
- **TopicCommand**: Nested subcommands enum.
- **Args**: Structs for command parameters (Args pattern).

```rust
#[derive(Parser, Debug)]
#[command(version)]
pub struct CliCmd {
    #[command(subcommand)]
    pub command: Option<CliSubCmd>,
}

#[derive(Subcommand, Debug)]
pub enum CliSubCmd {
    #[command(subcommand)]
    Topic(TopicCommand),
}

#[derive(Subcommand, Debug)]
pub enum TopicCommand {
    DoSomething(DoSomethingArgs),
}

#[derive(Args, Debug)]
pub struct DoSomethingArgs {
    pub name: String,
}
```

### 2. `src/cli/executor.rs`

Parses arguments and routes to the topic executor.

```rust
use clap::Parser as _;

pub fn execute() -> Result<()> {
    let cli_cmd = CliCmd::parse();

    let Some(sub_cmd) = cli_cmd.command else {
        println!("Welcome message...");
        return Ok(());
    };

    let res = match sub_cmd {
        CliSubCmd::Topic(command) => exec_topic::exec_command(command),
    };

    res?;

    Ok(())
}
```

### 3. `src/main.rs`

The entry point simply calls the CLI executor.

```rust
fn main() -> Result<()> {
    cli::execute()?;

    Ok(())
}
```

### 4. `src/cli/exec_<sub_command>.rs`

When the sub_command become substantial, more than a dozen, then we split them into `exec_<sub_command>.rs` to be more foxued. 

This file handles the translation from CLI-specific types (like `String` or `Vec<String>`) to domain types (like `SPath` or custom enums).

- Implement a public `exec_command` function with the signature: 
  `pub fn exec_command(command: TopicCommand) -> Result<()>`
- Use private helper functions for each subcommand's logic.
- Handle `println!` or `eprintln!` output here.

### 5. `src/handlers/`

Handlers should be "CLI-clean". They should not depend on `clap` or any CLI-specific structures.

- Handlers should take domain types as arguments (e.g., `impl AsRef<SPath>` instead of `&str` for paths).
- They should return `Result<T>` and avoid side effects like printing to stdout when possible.
- This allows handlers to be used in other contexts (e.g., as a library or in tests) without CLI overhead.

### 6. Type Conversions

Perform type conversions and validations as early as possible in the `exec_<sub_command>.rs` layer.

- Flatten optional CLI arguments into clean internal types.
- Validate file existence before calling handlers.
