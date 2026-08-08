# FAQ

## Is ENG natural-language programming?

No. ENG is structured English. The syntax is designed to read naturally while still being deterministic enough for a handwritten parser.

## Does every statement need a period?

Yes. A normal ENG statement ends with `.`. Block constructs also end with `End.`.

## Are keywords case-sensitive?

No. The lexer recognizes keywords case-insensitively.

## Can I use normal symbols such as `+` and `>`?

The language grammar is designed around English operator phrases such as `plus` and `is greater than`. Use the documented ENG forms rather than assuming conventional punctuation operators are accepted.

## Are integers and floats different types?

No. Numbers are represented as `f64`.

## How do I concatenate strings?

Use `plus`:

```engling
Print "Hello, " plus "world.".
```

## How do I create a list?

```engling
Make a list called items.
```

## Are list indexes zero-based?

The language exposes ordinal positions beginning at one. `first` and `1st` refer to the first item. Internally the VM converts that position to a zero-based vector index.

## How do I import a module?

```engling
Import math_helpers.
```

or:

```engling
From math_helpers use square.
```

The module file is `math_helpers.eng`.

## Where does ENG look for modules?

It first checks beside the importing source file, then directories listed in `ENGLING_PATH`.

## Does ENG have a package manager?

Not in the supplied v0.1.0 source tree. The current CLI exposes `run` and the no-argument REPL.

## Why does an unknown word sometimes get a suggestion?

The lexer compares sufficiently similar words against a keyword list. This catches common typos without preventing normal identifiers.

## How do I enable GUI support?

Build with:

```bash
cargo build --features ui
```

and run with:

```bash
eng run program.eng --ui
```

## What should I read first?

Start with [Getting Started](getting-started.md), then [Syntax](syntax.md), [Variables](variables.md), [Operators](operators.md), and [Control Flow](conditions.md).
