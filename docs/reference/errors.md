# Errors and Debugging

ENG has four main error categories:

- Lexer errors
- Parse errors
- Runtime errors
- Module errors

Position-aware lexer and parser errors are rendered with `miette`.

## Lexer errors

A lexer error means the source could not be converted into tokens.

For example, an unterminated string:

```engling
Print "hello.
```

The lexer reports the line and column and identifies the problem.

Unknown words may also receive spelling suggestions. For example, a typo close to `print` can produce a message such as:

```text
Unknown word 'pritn'. Did you mean 'print'?
```

## Parse errors

A parse error means the words were recognized but did not match an ENG sentence template.

Common causes:

- missing `.`
- missing `then` in an `If`
- missing `End.`
- incorrect function syntax
- malformed list syntax
- incorrect import syntax

The error includes a source location and suggests checking the grammar.

## Runtime errors

Runtime errors happen after parsing, for example:

```engling
Let a be "hello".
Let b be 5.
Print a plus b.
```

This attempts to add incompatible values.

Other runtime errors include:

- numeric operations on non-numbers
- invalid list indexes
- calling an undefined function
- calling a non-function
- wrong argument count
- list operations on non-lists

## Module errors

Module errors include missing modules, missing imported names, and circular imports.

## Debugging strategy

When a program fails:

1. Read the reported line and column.
2. Compare the statement against the corresponding documentation page.
3. Reduce the program to the smallest failing statement.
4. Test that statement in the REPL.
5. Check the runtime value types involved.
