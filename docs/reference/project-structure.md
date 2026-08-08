# Project Structure

The repository separates the language pipeline into small Rust modules.

```text
src/
  main.rs
  lib.rs
  error.rs
  token.rs
  lexer.rs
  ast.rs
  parser.rs
  bytecode.rs
  compiler.rs
  value.rs
  scope.rs
  vm.rs
  runtime.rs
  cli.rs
  repl.rs
  ui/
```

## `main.rs`

The binary entry point. The Cargo binary is named `eng`.

## `cli.rs`

Defines the command-line interface and dispatches `run` or the REPL.

## `lexer.rs` and `token.rs`

The lexer converts source text into `Token`s. `TokenKind` contains keywords, literals, operators, punctuation, and identifiers.

## `parser.rs` and `ast.rs`

The recursive-descent parser converts tokens into the typed AST.

## `compiler.rs` and `bytecode.rs`

The compiler turns AST statements and expressions into bytecode instructions.

## `vm.rs`

The virtual machine executes bytecode using a value stack, call frames, and scopes.

## `value.rs`

Defines runtime values such as numbers, strings, booleans, lists, functions, and `nothing`.

## `scope.rs`

Maintains global and local bindings.

## `runtime.rs`

Connects source loading, lexing, parsing, module loading, compilation, and VM execution.

## `ui/`

Contains the optional GUI bridge enabled by the `ui` feature.

## `examples/`

Contains runnable `.eng` examples.

## `tests/`

Contains integration and probe tests for language behavior.
