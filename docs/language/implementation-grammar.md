# Engling Language Grammar

Engling is a programming language whose syntax is structured English. Every sentence
ends with a period (`.`). Keywords are case-insensitive. Statements are produced from
fixed templates rather than free-form English, so the grammar is unambiguous and
totally driven by a hand-written recursive-descent parser.

This document describes the Engling v0.1.0 grammar — the surface syntax accepted by
the interpreter, with EBNF and example sentences for every construct.

---

## 1. Top-level structure

```ebnf
program        ::= statement* EOF
statement      ::= decl | assign | print | if_stmt
                 | repeat_stmt | while_stmt
                 | func_def    | func_call
                 | list_decl   | list_add | list_get_stmt | list_set_stmt
                 | module_decl | import_stmt | import_from_stmt
                 | window_decl | widget_decl | event_handler   (* UI feature *)
                 | set_label_text                             (* UI feature *)
```

Every statement ends with a period. Multi-line blocks (such as `If`, `Repeat`,
`While`, function bodies) end with `End.`.

---

## 2. Variables and assignment

```ebnf
decl       ::= ("Let"|"Make") IDENT "be" expr "."
assign     ::= "Set" IDENT "to" expr "."
```

```engling
Let x be 5.
Make greeting be "hello".
Set x to x plus 1.
```

`Make` is a synonym for `Let` when the right-hand side is not `a list` or
`a window`. The interpreter routes the token in the lexer; the parser only sees
the canonical `Let` form.

---

## 3. Print

```ebnf
print      ::= ("Print"|"Show"|"Display") expr "."
```

```engling
Print "Hello, world.".
Show 42.
Display count.
```

---

## 4. Expressions

```ebnf
expr            ::= logic_or
logic_or        ::= logic_and ("or" logic_and)*
logic_and       ::= comparison ("and" comparison)*
comparison      ::= addition (("is" ("equal"|"not" "equal"|"greater" "than"
                                  ("or" "equal")?
                                |"less" "than" ("or" "equal")?)) addition)*
addition        ::= multiplication (("plus"|"minus") multiplication)*
multiplication  ::= unary (("multiplied" "by"|"divided" "by"|"modulo") unary)*
unary           ::= primary
primary         ::= NUMBER | STRING | "true" | "false" | IDENT
                 |  "Run" IDENT ("with" arg_list)?
                 |  "Get" "the" ordinal "item" "of" IDENT
```

### Comparison forms

| Sentence                       | Operator     |
| ------------------------------ | ------------ |
| `x is equal to 5.`             | `==`         |
| `x is not equal to 5.`         | `!=`         |
| `x is greater than 5.`         | `>`          |
| `x is less than 5.`            | `<`          |
| `x is greater than or equal to 5.` | `>=`     |
| `x is less than or equal to 5.`| `<=`         |

### Arithmetic forms

| Sentence                            | Operator |
| ----------------------------------- | -------- |
| `x plus y.`                         | `+`      |
| `x minus y.`                        | `-`      |
| `x multiplied by y.`                | `*`      |
| `x divided by y.`                   | `/`      |
| `x modulo y.`                       | `%`      |

String concatenation is also `+` (`"Hello, " plus "world."`).

---

## 5. Control flow

```ebnf
if_stmt       ::= "If" expr "," "then" block
                  ("Otherwise" block)?
                  "End" "."
repeat_stmt   ::= "Repeat" expr "times" block "End" "."
while_stmt    ::= "While" expr block "End" "."
block         ::= statement*
```

```engling
If age is greater than 18, then
  Print "Adult".
Otherwise
  Print "Minor".
End.

Repeat 5 times
  Print "hi".
End.

While count is less than 10
  Set count to count plus 1.
End.
```

The `If` requires a comma after the condition and the word `then` before the
block. `Otherwise` is optional.

---

## 6. Functions

```ebnf
func_def      ::= "Define" ("a"|"an")? "function" "called" IDENT
                  "that" "takes" param_list
                  ( "and" "returns" expr "."
                  | body_block "End" "." )
func_call     ::= ("Run"|"Call") IDENT ("with" arg_list)? "."
param_list    ::= "nothing" | IDENT (("and"|",") IDENT)*
arg_list      ::= expr (("and"|",") expr)*
```

```engling
Define a function called greet that takes name and returns "Hello, " plus name.

Run greet with "world".

Define a function called increment that takes nothing
  Set count to count plus 1.
End.
```

Calls may also appear inside expressions to use the returned value:

```engling
Let greeting be Run greet with "world".
```

---

## 7. Lists

```ebnf
list_decl      ::= "Make" ("a"|"an") "list" "called" IDENT "."
list_add       ::= "Add" expr "to" IDENT "."
list_get_stmt  ::= "Get" "the" ordinal "item" "of" IDENT   (* expression form *)
list_set_stmt  ::= "Set" "the" ordinal "item" "of" IDENT "to" expr "."
list_length    ::= "the" "length" "of" IDENT              (* expression form *)
ordinal        ::= "first"|"second"|"third"|"fourth"|"fifth"
                 |  NUMBER ("st"|"nd"|"rd"|"th")?
```

`Get the third item of scores` is an expression that yields the value at index 2
(zero-based). The ordinal `1st` becomes index 0, so `1st` is the first item.

```engling
Make a list called scores.
Add 10 to scores.
Add 20 to scores.
Set the first item of scores to 5.
Let first_score be Get the first item of scores.
Print the length of scores.
```

---

## 8. Modules

```ebnf
module_decl      ::= "Create" ("a"|"an") "module" "called" IDENT "."
import_stmt      ::= "Import" IDENT "."
import_from_stmt ::= "From" IDENT "use" IDENT (("and"|",") IDENT)* "."
```

A module is any `.eng` file whose top-level `Define`-bindings are exported
automatically. `Import math_helpers` imports all top-level definitions from
`math_helpers.eng` (resolved relative to the importing file, then via
`ENGLING_PATH`). `From math_helpers use square_root` imports only the named
identifiers.

```engling
Import math_helpers.
From math_helpers use square_root.
```

---

## 9. UI (feature `ui`)

```ebnf
window_decl      ::= "Make" ("a"|"an") "window" "called" IDENT
                     "titled" STRING "."
widget_decl      ::= ("Add"|"Make" ("a"|"an"))
                     ("button"|"label"|"text" "field")
                     "to" IDENT "labeled" STRING "."
event_handler    ::= "When" "the" STRING "button" "is" "clicked"
                     "," "run" IDENT "."
set_label_text   ::= "Set" "the" ("label" "text")? "of" IDENT "to" expr "."
```

```engling
Make a window called app titled "Counter".
Add a label to app labeled "Count: 0".
Add a button to app labeled "Increment".
Define a function called increment that takes nothing
  Set count to count plus 1.
  Set the label text of count_label to count.
End.
Let count be 0.
When the Increment button is clicked, run increment.
```

Run the program with `eng run app.eng --ui` (built with `--features ui`).

---

## 10. Reserved words

The lexer is permissive. Any otherwise-unrecognized word is treated as an
identifier. Reserved words are listed below:

```
let, set, make, be, to, print, show, display,
true, false,
if, otherwise, end, repeat, times, while, then,
define, function, called, takes, returns, run, call, with, a, an, nothing,
list, add, get, the, item, of, first, second, third, fourth, fifth,
st, nd, rd, th,
import, from, use, module, create,
window, titled, button, label, text, field, when, clicked, labeled,
plus, minus, multiplied, divided, by, modulo,
is, equal, not, greater, less, than, and, or
```

---

## 11. Punctuation

- `.` terminates every statement.
- `,` separates the condition from `then` in `If`, and joins arguments.
- `"..."` denotes a string literal.
- `#` starts a line comment to end of line.

---

## 12. Synonyms (lexer only)

| Canonical | Aliases          |
| --------- | ---------------- |
| `Let`     | `make` (non-list) |
| `Print`   | `show`, `display` |
| `Run`     | `call`            |
| `Define`  | `create` (non-module) |

The parser only sees canonical tokens. Aliases are normalized in the lexer.
