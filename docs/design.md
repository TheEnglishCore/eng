# Engling Language Design

This document explains the design decisions behind Engling v0.1.0 — *why* the
language is shaped the way it is, the tradeoffs it accepts, and the
implementation path that makes it run on a small Rust interpreter.

---

## 1. Goals

Engling is for people who can read English but have not learned a programming
language. The tagline is **"programming in plain English"** — not "natural
language processing". Two consequences:

1. The surface syntax is *structured English*. Programs read roughly like a
   human-coded recipe, but every sentence fits a known template.
2. The implementation is a *bytecode VM*, not an LLM. Predicability beats
   flexibility: every accepted program is exactly one AST, exactly one
   behavior.

The grammar is intentionally not Turing-complete in *parse forms* — we trade
language expressiveness for compile-time determinism.

---

## 2. English vs. parseability

The hardest design question is: *how much English is "plain enough"?*

| Lever                | Choice                                              |
| -------------------- | --------------------------------------------------- |
| Word order           | Fixed templates per statement form                  |
| Statement boundary   | Every statement ends with a period (`.`)            |
| Synonyms             | Whitelisted alias→canonical mappings in the lexer    |
| Grammar ambiguity    | Resolved by greedy multi-token operators in the lexer |
| Free-form sentences  | Rejected — must match a template                    |

The period terminator is the single most important decision. It lets numbers
(`5.`) and end-of-statement dots coexist without lookahead into whitespace.

### What we *don't* support

- Imperative mood variation: `Please print 5.` is rejected (no `please` keyword).
- Syntactic forms we have not enumerated: `Whisper 5.` is rejected as an unknown
  word.
- Past-tense runtime effects: `Set x to 5.` does not remember the old value of
  `x`; the language is single-assignment in habit, not in semantics.

These are features, not restrictions.

---

## 3. Lexer-driven synonyms

Adding a new synonym (e.g. `display` for `print`) means one line in the
lexer — `match lower.as_str() { "display" => TokenKind::Print, ... }`. The
parser never sees the alias. This keeps the grammar lean while letting
programmers pick their dialect.

The full synonym table is in `docs/GRAMMAR.md` §12.

---

## 4. Pipeline architecture

```
Source → Lexer → Parser → Compiler → VM
                  ↑                       ↑
              ModuleLoader            UI Bridge (opt-in)
```

- **Lexer** — pure stream. Resolves synonyms, supports comments, recognizes
  multi-word operators `multiplied by`, `is greater than or equal to` via
  greedy lookahead.
- **Parser** — recursive-descent, 1-token lookahead, produces a typed AST.
- **Compiler** — single-pass AST → bytecode. Tracks the loop counter for
  `repeat`/`while` using a hidden variable name (`__repeat_0`, etc.).
- **VM** — small stack machine with explicit call frames and a scope stack.
  Handles integers (stored as `f64`), strings, booleans, lists, and
  closures.

This is the same shape as Crafting Interpreters' clox, scoped to fit a
single-author repo.

---

## 5. Scope and closures

Engling has lexical scope. The `ScopeStack` is a stack of `HashMap`s for
locals, plus a single `globals` map. Functions are first-class values and
carry the chunk they were compiled from. They capture the runtime scope at
call time via the call frame, not the lexical scope at definition time —
this is the simplest workable model that supports recursion.

When a function is called:

1. Push a new scope frame.
2. Bind the parameters in the frame.
3. Push a new call frame onto the VM stack.
4. Run until the inner `Return` instruction pops the frame.
5. Pop the scope frame.

The return value is left on the stack for the caller to consume.

---

## 6. Errors as a feature

The interpreter uses `thiserror` enums (`EnglingError::Lex`, `Parse`, `Runtime`,
`Module`) decorated with `miette::Diagnostic` codes (`engling::lex`,
`engling::parse`, `engling::runtime`, `engling::module`) so the pretty-printer
in `miette::Report` can show a labeled snippet plus a one-line "help" hint for
each. Errors that carry a position (`Lex`, `Parse`) include line and column
from the originating token.

Unknown words are checked against a Levenshtein distance of 2 against the
keyword list, so `pritn` becomes a suggestion:
`Unknown word 'pritn'. Did you mean 'print'?`.

The grammar's strictness is complemented by the error messages: every
unexpected token is named, and the parser always tells you what it expected
at that position.

---

## 7. UI as an optional feature

The UI bridge (`eframe` + `egui`) is gated behind `--features ui`. The
default build is a pure CLI binary that runs on Termux, CI servers, and
headless containers without graphics.

The UI integration is small:

- Top-level `Make a window`, `Add a ...`, `When the X button is clicked`
  statements are intercepted at runtime before the bytecode runs and
  registered in a static `UI_STATE`.
- The program runs (assignments, function definitions, etc.) on the CLI VM.
- After the program completes, if any window was declared, `eframe` takes
  over and runs the event loop.
- Event handlers call back into the VM via the same `Call` path the
  interpreter uses, so all closures and nested functions work correctly.

The split keeps the interpreter core unmodified and the GUI purely additive.

---

## 8. Modules

Modules are just files. A module is a `.eng` file whose top-level
`Define` statements are exported automatically. There is no explicit `export`
keyword — defining a function at the top level is the act of exporting.

`Import math_helpers` resolves `./math_helpers.eng` relative to the
importing file, then falls back to each directory in `ENGLING_PATH`
(`;` on Windows, `:` on Unix). Cached imports are reused; circular imports
are detected and rejected.

The persistent VM across files is key: when a module is loaded, the loader
runs the file in its own throwaway VM, then copies its globals into the
caller's VM as namespace bindings.

---

## 9. Numbers

Numbers are `f64` internally. They are parsed as `f64` to support both
integer and floating-point literals, including scientific notation. The
arithmetic operators are aware of this; `5` and `5.0` are the same value.

For a 0.1.0 release the language does not distinguish int from float. This
was a deliberate choice: it removes a whole class of parse errors
("5 plus 5.0: int + float?") at the cost of legal implicit widening.

---

## 10. What is deliberately not in 0.1.0

- Classes, inheritance, prototypes.
- Exceptions / try-catch.
- Async / coroutines.
- A package manager.
- LSP server.

Each is a real feature; none belongs in a language whose first job is to be
readable by humans who have never programmed.

---

## 11. Roadmap

Beyond 0.1.0:

1. A `match` / `case` statement for pattern-free dispatch.
2. Strings with interpolation: `"Hello, {name}."`.
3. A standard library file shipped via `ENGLING_PATH`.
4. Bytecode verifier + safelist for embedded use.
5. WASM target so the interpreter runs in a browser.
