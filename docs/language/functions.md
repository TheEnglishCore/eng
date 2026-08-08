# Functions

Functions are defined with `Define` and can accept parameters and optionally return an expression.

## Function with a return expression

```engling
Define a function called square that takes x and returns x multiplied by x.

Print Run square with 5.
```

The function call produces a value that can be used in another expression.

## Multiple parameters

```engling
Define a function called add that takes a and b and returns a plus b.

Print Run add with 2 and 3.
```

Arguments can also be separated with commas where accepted by the parser:

```engling
Print Run add with 2, 3.
```

## No parameters

Use `nothing`:

```engling
Define a function called hello that takes nothing
    Print "Hello".
End.

Run hello.
```

A body-style function without a return expression produces `nothing`.

## Body functions

A function can contain normal statements:

```engling
Define a function called greet that takes name
    Print "Hello, " plus name.
End.

Run greet with "Awi".
```

## Calling functions

`Run` and its lexer alias `Call` use the same canonical token:

```engling
Run greet with "Awi".
```

A call can be used as an expression when a return value is needed:

```engling
Let result be Run square with 5.
```

## Argument errors

The VM checks argument counts. Calling a function with the wrong number of arguments produces a runtime error such as:

```text
Function 'square' expects 1 arguments, got 2
```

Calling a name that is not a function also produces a runtime error.
