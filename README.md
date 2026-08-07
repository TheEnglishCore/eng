# Engling v0.1.0

Programming in plain English. This repo is a bytecode VM interpreter for the
Engling language, written in Rust.

## Quick start

```bash
# Run a program
cargo run -- run examples/01_hello.eng

# Try all 24 non-UI examples
for f in examples/0[1-9]_*.eng examples/1[0-9]_*.eng examples/2[0-6]_*.eng; do
    cargo run -- run "$f"
done

# Build with GUI support and run the counter demo
cargo build --features ui --release
./target/release/eng run examples/25_window_counter.eng --ui

# REPL
cargo run
```

## Language at a glance

```engling
Let greeting be "Hello, world.".
Print greeting.

Let answer be 6 multiplied by 7.
Print answer.

If answer is greater than 40, then
    Print "Big number".
Otherwise
    Print "Small number".
End.

Repeat 3 times
    Print "hi".
End.

Define a function called square that takes x and returns x multiplied by x.
Print Run square with 5.
```

See [`docs/GRAMMAR.md`](docs/GRAMMAR.md) for the full grammar and
[`docs/DESIGN.md`](docs/DESIGN.md) for design rationale.

## Layout

```
src/
  main.rs, lib.rs
  error.rs
  token.rs, lexer.rs
  ast.rs, parser.rs
  bytecode.rs, compiler.rs
  value.rs, scope.rs, vm.rs
  runtime.rs, repl.rs, cli.rs
  ui/                  # gated behind --features ui
docs/
  GRAMMAR.md, DESIGN.md
examples/              # 26 example programs (24 non-UI + 2 UI)
tests/
  integration_test.rs  # runs every fixture + edge cases
```

## Building the optional UI feature

```bash
cargo build --features ui
```

Without `--features ui`, the interpreter runs everywhere Rust does — including
Termux, CI containers, and WASM targets — and UI statements (`Make a window`,
`Add a button`, `When the X button is clicked`) raise parse errors.

## License

Dual-licensed under MIT or Apache-2.0, at your option.
