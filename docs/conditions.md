# Conditions

Use `If` to conditionally execute a block.

## Basic `If`

```engling
Let x be 10.

If x is greater than 5, then
    Print "Big".
End.
```

The comma and `then` are part of the syntax.

## `Otherwise`

```engling
Let score be 75.

If score is greater than or equal to 60, then
    Print "Pass".
Otherwise
    Print "Fail".
End.
```

Only one branch executes.

## Combining conditions

```engling
Let age be 25.
Let has_id be true.

If age is greater than 18 and has_id, then
    Print "Allowed".
End.
```

`and` combines boolean expressions. `or` is also supported.

## Truthy values

Conditions are evaluated using the runtime value's truthiness. See [Data Types](data-types.md).

## Common syntax mistake

This is invalid:

```engling
If x is greater than 5
    Print "Big".
End.
```

The `If` grammar requires:

```engling
If x is greater than 5, then
```
