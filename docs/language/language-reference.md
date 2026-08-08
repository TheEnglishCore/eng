# Language Reference

This page is a compact reference for ENG v0.1.0.

## Declaration

```engling
Let name be expression.
Make name be expression.
```

## Assignment

```engling
Set name to expression.
```

## Output

```engling
Print expression.
Show expression.
Display expression.
```

## Literals

```engling
42
3.14
"hello"
true
false
```

## Arithmetic

```engling
a plus b
a minus b
a multiplied by b
a divided by b
a modulo b
```

## Comparison

```engling
a is equal to b
a is not equal to b
a is greater than b
a is less than b
a is greater than or equal to b
a is less than or equal to b
```

## Logic

```engling
a and b
a or b
```

## Conditional

```engling
If condition, then
    statements
Otherwise
    statements
End.
```

## Loops

```engling
Repeat count times
    statements
End.
```

```engling
While condition
    statements
End.
```

## Functions

```engling
Define a function called name that takes x and returns expression.
```

or:

```engling
Define a function called name that takes x
    statements
End.
```

## Function calls

```engling
Run name.
Run name with value.
Run name with value1 and value2.
```

## Lists

```engling
Make a list called name.
Add value to name.
Get the first item of name.
Set the first item of name to value.
Print the length of name.
```

## Modules

```engling
Import module.
From module use name.
```

## UI

When built with `--features ui`:

```engling
Make a window called app titled "Example".
Add a label to app labeled "Hello".
Add a button to app labeled "OK".
When the OK button is clicked, run handler.
```

## Comments

```engling
# comment
```
