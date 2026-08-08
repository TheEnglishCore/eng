# Contributing

ENG is organized as a small Rust interpreter. Changes are easiest to review when they are limited to the subsystem they actually affect.

## Typical workflow

1. Make a change.
2. Run `cargo fmt`.
3. Run `cargo test`.
4. Run a relevant example with `cargo run -- run ...`.
5. Update the documentation if syntax or runtime behavior changed.

## Adding syntax

A new language construct generally involves:

1. adding or adjusting a `TokenKind`
2. teaching the lexer to recognize the words
3. adding parser logic
4. adding an AST variant if needed
5. compiling the AST to bytecode
6. adding VM/runtime behavior
7. adding tests
8. documenting the syntax

Do not add a keyword if an identifier can safely represent the same concept.

## Parser changes

ENG uses a handwritten recursive-descent parser. Keep new grammar forms explicit and unambiguous.

Every new statement should have a clear terminating period and block constructs should have a clear `End.` boundary.

## Runtime changes

Runtime errors should use `EnglingError::runtime(...)` so they remain consistent with the existing error system.

Module problems should use `EnglingError::module(...)`.

## UI changes

GUI functionality is behind the `ui` Cargo feature. Avoid making the default interpreter depend on the GUI path.

## Documentation

When changing syntax, update both the grammar/reference documentation and at least one runnable example.
