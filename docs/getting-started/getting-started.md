# Getting Started

## Build from source

ENG is a Rust project. With Rust installed, build it with:

```bash
cargo build --release
```

The executable is named `eng`.

Run a program during development:

```bash
cargo run -- run examples/01_hello.eng
```

## Your first program

Create `hello.eng`:

```engling
Let name be "Awi".
Print "Hello, " plus name.
```

Run it:

```bash
eng run hello.eng
```

Output:

```text
Hello, Awi.
```

## A small program

```engling
Let score be 75.

If score is greater than or equal to 60, then
    Print "Pass".
Otherwise
    Print "Fail".
End.
```

The comma before `then` is required by the `If` grammar.

## Important syntax rules

1. Statements normally end in `.`.
2. Blocks end in `End.`.
3. Keywords are case-insensitive.
4. Strings use double quotes.
5. Comments begin with `#` and continue to the end of the line.
6. Operators use English phrases such as `plus`, `multiplied by`, and `is greater than`.

## Running the examples

The repository includes example programs under `examples/`. They cover arithmetic, strings, booleans, control flow, functions, lists, modules, and the optional UI.
