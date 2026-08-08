# Interpreter Architecture

ENG follows a conventional source-to-bytecode pipeline:

```text
Source
  |
  v
Lexer
  |
  v
Tokens
  |
  v
Parser
  |
  v
AST
  |
  v
Compiler
  |
  v
Bytecode
  |
  v
VM
```

Modules are handled by the runtime around the parsing/execution pipeline.

## Lexer

The lexer recognizes keywords case-insensitively and normalizes supported aliases to canonical token kinds.

It also recognizes multi-word operator phrases such as:

```text
multiplied by
divided by
is greater than
is less than or equal to
```

## Parser

The parser is handwritten recursive descent. ENG intentionally uses fixed templates instead of attempting general natural-language understanding.

This makes programs predictable and errors diagnosable.

## AST

The AST contains explicit statement variants for variables, assignments, printing, control flow, functions, lists, imports, modules, and optional UI declarations.

## Compiler

The compiler emits stack-machine bytecode.

For example, an addition expression loads both operands and emits an `Add` instruction.

`Repeat` is compiled using an internal counter variable and jump instructions.

## VM

The VM maintains:

- a value stack
- call frames
- a scope stack
- a configurable print callback

Functions push a new scope frame and call frame. The return value remains available to the caller.

## Runtime

`runtime.rs` creates the module loader, lexes and parses source, handles imports, compiles the program, and executes the resulting bytecode.

## UI

UI statements are intercepted by the optional UI layer before normal bytecode execution. This keeps GUI functionality out of the default interpreter path.
