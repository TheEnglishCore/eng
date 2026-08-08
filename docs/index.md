# ENG Documentation

**ENG** (the executable is `eng`) is a small programming language whose syntax is written as structured English.

> Programming in plain English — with a fixed grammar that the interpreter can parse deterministically.

This documentation describes the syntax implemented by the current Engling v0.1.0 interpreter.

## Quick start

Run a source file:

```bash
cargo run -- run examples/01_hello.eng
```

Or, after building the binary:

```bash
eng run program.eng
```

Start the REPL by running `eng` without a subcommand:

```bash
eng
```

A minimal program:

```eng
Let greeting be "Hello, world.".
Print greeting.
```

Every statement ends with a period (`.`). Blocks such as `If`, `Repeat`, `While`, and function bodies end with `End.`.

## What ENG supports

- Variables and assignment
- Numbers, strings, booleans, lists, functions, and `nothing`
- Arithmetic and comparisons
- Boolean `and` / `or`
- `If` / `Otherwise`
- `Repeat` and `While`
- Functions with parameters and return expressions
- Lists with ordinal indexing
- Modules and selective imports
- An optional GUI feature using `egui`/`eframe`
- A bytecode compiler and stack-based virtual machine

## Documentation map

- [Getting Started](getting-started.md)
- [CLI](cli.md)
- [Syntax](syntax.md)
- [Variables](variables.md)
- [Data Types](data-types.md)
- [Operators](operators.md)
- [Conditions](conditions.md)
- [Loops](loops.md)
- [Functions](functions.md)
- [Lists](lists.md)
- [Strings](strings.md)
- [Modules and Imports](modules.md)
- [REPL](repl.md)
- [Errors and Debugging](errors.md)
- [GUI](gui.md)
- [Project Structure](project-structure.md)
- [Architecture](architecture.md)
- [Grammar Reference](grammar.md)
- [Examples](examples.md)
- [FAQ](faq.md)

## Version note

The syntax documented here is based on the source tree supplied with this documentation. ENG is intentionally strict: it accepts known sentence templates rather than arbitrary English prose.
