# Command Line

The executable is named `eng`.

## Run a file

```bash
eng run program.eng
```

The source file is read from disk and executed by the interpreter.

The repository also supports:

```bash
cargo run -- run examples/01_hello.eng
```

when developing from source.

## GUI mode

GUI support is optional:

```bash
cargo build --features ui --release
```

Then:

```bash
eng run examples/25_window_counter.eng --ui
```

If the binary was built without the `ui` feature, `--ui` does not enable the GUI; the program prints a warning explaining that the feature must be compiled in.

## Start the REPL

Run `eng` without a subcommand:

```bash
eng
```

This starts the interactive REPL.

## Current command set

The CLI currently exposes the `run` subcommand and the no-argument REPL. Package-manager commands are not part of the supplied v0.1.0 CLI.

If you see documentation claiming that `eng install`, `eng remove`, or `eng update` already exist, that documentation does not match this source tree.
