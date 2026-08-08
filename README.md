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

## Package manager

Engling V1 ships with a small, deliberate package manager that
supports both **official** and **community** packages.

```bash
# Official packages
eng install colors

# Community packages (GitHub or direct archive URL)
eng install https://github.com/Alice/colors
eng install https://example.com/colors-1.0.0.engpkg

# Inspect / manage
eng list
eng remove colors
eng search color
eng update
```

The official registry defaults to:

```text
https://raw.githubusercontent.com/TheEnglishCore/eng-packages/main/registry.json
```

Override with `ENGLING_REGISTRY=<url>`. Packages land under
`~/.engling/packages/` on Linux, macOS, and Termux. Override with
`ENGLING_PACKAGES_DIR`.

See [`docs/PACKAGES.md`](docs/PACKAGES.md) for the full spec — manifest
format, security guarantees, dependency handling, module resolution
order, and how to publish your own community package.

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
  package/             # V1 package manager
    mod.rs, source.rs, registry.rs, manifest.rs, version.rs,
    fetcher.rs, installer.rs, store.rs, commands.rs
  ui/                  # gated behind --features ui
docs/
  GRAMMAR.md, DESIGN.md, PACKAGES.md
examples/              # 26 example programs (24 non-UI + 2 UI)
tests/
  integration_test.rs  # runs every fixture + edge cases
  package_manager.rs   # exercises the package manager end-to-end
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