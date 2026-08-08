# REPL

Running `eng` without a subcommand starts the interactive Read-Eval-Print Loop.

```bash
eng
```

When developing from source:

```bash
cargo run
```

The REPL uses the Rust `rustyline` crate for interactive input.

## What to try

Simple expressions/statements can be entered interactively:

```engling
Let x be 5.
Print x.
```

You can also experiment with functions and arithmetic.

## When to use the REPL

The REPL is useful for:

- checking syntax quickly
- testing arithmetic
- experimenting with values
- trying a small function before putting it in a `.eng` file

For multi-line programs, a `.eng` file is generally easier because block constructs require their complete `End.` terminator.

## REPL vs files

File execution uses the module loader with the directory of the source file as the module base directory. The REPL executes source through the runtime using the current directory as its module base.
