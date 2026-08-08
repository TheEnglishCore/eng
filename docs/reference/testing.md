# Testing ENG Programs

The repository uses Rust integration tests to verify interpreter behavior.

## Run the test suite

```bash
cargo test
```

## What is tested

The supplied tests cover:

- hello-world execution
- printed output
- arithmetic
- string concatenation
- booleans
- comparisons
- logical operators
- `If` / `Otherwise`
- `Repeat`
- `While`
- functions
- lists
- module loading
- error cases

## Add a regression test

When changing the interpreter, add a small test that demonstrates the intended behavior.

A typical output test constructs a VM with a custom printer, executes source, and compares the emitted strings.

Example source:

```engling
Let x be 5.
Print x.
```

Expected output:

```text
5
```

## Test examples manually

A quick manual test is:

```bash
cargo run -- run examples/01_hello.eng
```

For a larger check, run the repository's example files individually or use the test suite.

## Why tests matter

ENG's parser is deliberately template-driven. A small syntax change can affect multiple sentence forms, so regression tests help preserve existing grammar while new features are added.
