# Grammar Reference

This is the practical grammar reference for ENG v0.1.0.

## Program

```text
program ::= statement* EOF
```

## Variables

```text
Let IDENT be expression.
Make IDENT be expression.
Set IDENT to expression.
```

Example:

```engling
Let x be 5.
Set x to x plus 1.
```

## Output

```text
Print expression.
Show expression.
Display expression.
```

`Show` and `Display` are lexer aliases for `Print`.

## Expressions

Conceptually:

```text
expression
  -> or
  -> and
  -> comparison
  -> addition
  -> multiplication
  -> primary
```

Primary values include numbers, strings, booleans, identifiers, function calls, and list access.

## Conditions

```text
If expression, then
    statements
Otherwise
    statements
End.
```

`Otherwise` is optional.

## Repeat

```text
Repeat expression times
    statements
End.
```

## While

```text
While expression
    statements
End.
```

## Functions

Return form:

```text
Define a function called NAME that takes PARAMETERS and returns EXPRESSION.
```

Body form:

```text
Define a function called NAME that takes PARAMETERS
    statements
End.
```

`nothing` represents an empty parameter list.

## Calls

```text
Run NAME.
Run NAME with ARGUMENTS.
```

`Call` is accepted as a lexer alias for `Run`.

## Lists

```text
Make a list called NAME.
Add EXPRESSION to NAME.
Get the ORDINAL item of NAME.
Set the ORDINAL item of NAME to EXPRESSION.
Print the length of NAME.
```

## Modules

```text
Import NAME.
From NAME use IDENTIFIER.
From NAME use IDENTIFIER and IDENTIFIER.
```

## Comments

```text
# comment
```

Comments run to the end of the line.
