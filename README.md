# rust10x

rust10x is an evolving knowledge base for production-grade Rust development. It is structured and optimized for both human developers and AI coding agents.

## Structure

All content is organized under the `pack/` directory:

- `pack/guide/base/`: Core Rust best practices, conventions, and design patterns.
- `pack/guide/domain/`: Domain-specific practices, patterns, and architectures.
- `pack/samples/`: Self-contained, compilable micro-projects demonstrating specific design patterns in focused codebases.
- `pack/code-map.json`: A content map indexing all pack files, structured for use by automated context engines and AI coding harnesses such as AIPack and pro@coder.

## AI and Agent Integration

The `pack/` directory is designed to be provided directly to AI assistants. Agents can reference the structured guides, compilable sample projects, and the code map during planning and code generation to produce idiomatic, production-grade Rust.

## Using with AIPack / pro@coder

For those brave enough to use the AIPack and `pro@coder` coding harnesses:

Install [aipack](https://aipack.ai)

```
aip install pro@rust10x
```

Then, in the pro@coder `coder-prompt.md`, you can have pro@coder generate the code map.

```yaml
knowledge_globs: 
  - pro@rust10x/guide/**/*.md    # to get the markdown guides
  - pro@rust10x/samples/**/*.*  # to get the sample code

auto_context: 
  model: luna            # Medium/Cheap model enough on this one (gpt-5.6-luna good choice)
  code_map_model: lite   # Small/fast to code map each file (gemini flash lite the best for this)
  input_concurrency: 32  # (default 8)
  # enabled: false       # (default true)

model: luna-xhigh
```


Alternatively, you can point directly to the embedded code map:

```yaml
knowledge_globs: 
  - pro@rust10x/code-map.json
```

This uses the `rust10x` code map provider.
