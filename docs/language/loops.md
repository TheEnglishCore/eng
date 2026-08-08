# Loops

ENG has `Repeat` and `While`.

## Repeat a fixed number of times

```engling
Repeat 3 times
    Print "hi".
End.
```

Output:

```text
hi
hi
hi
```

The repeat count is evaluated once and stored internally as a counter. The body executes while the counter is greater than zero.

## Repeat using an expression

```engling
Let count be 3.

Repeat count times
    Print "hello".
End.
```

## While

```engling
Let count be 0.

While count is less than 3
    Print count.
    Set count to count plus 1.
End.
```

A `While` loop reevaluates its condition each iteration.

## Avoid accidental infinite loops

Make sure the state used by the condition changes:

```engling
Let count be 0.

While count is less than 3
    Set count to count plus 1.
End.
```

If `count` never changes, the condition can remain true forever.

## Nested loops

Because loop bodies are normal statement blocks, loops can be nested:

```engling
Repeat 2 times
    Repeat 3 times
        Print "x".
    End.
End.
```
