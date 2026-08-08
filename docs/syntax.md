# Syntax

ENG uses structured English. It is English-looking syntax with fixed parser templates, not unrestricted natural-language input.

## Statements

Examples:

```engling
Let x be 5.
Set x to 10.
Print x.
```

Each statement ends with a period.

## Blocks

Control-flow and function blocks use `End.`:

```engling
If x is greater than 5, then
    Print "large".
End.
```

## Comments

A `#` begins a line comment:

```engling
# This is a comment.
Let x be 5. # This comment follows a statement.
```

## Case

Keywords are recognized case-insensitively, so keyword casing does not change their meaning.

Identifiers are stored as identifiers by the lexer and should be kept consistently named for readability.

## Strings

Strings are enclosed in double quotes:

```engling
Let message be "Hello".
```

A newline inside a string is rejected as an unterminated string.

## Punctuation

- `.` terminates statements.
- `,` is required between an `If` condition and `then`, and can separate function arguments.
- `"` starts and ends a string.

## What is not valid

ENG does not accept arbitrary prose:

```engling
Please print the value.
```

The parser expects the known `Print expression.` template instead.
