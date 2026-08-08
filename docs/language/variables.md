# Variables

Variables are introduced with `Let` and changed with `Set`.

## Declare a variable

```engling
Let x be 5.
Let message be "hello".
Let enabled be true.
```

`Make` is also accepted for ordinary variable declarations:

```engling
Make greeting be "hello".
```

## Assign a new value

```engling
Let score be 10.
Set score to 20.
Set score to score plus 5.
```

`Set` evaluates the expression on the right and stores the result under the variable name.

## Expressions on assignment

```engling
Let width be 4.
Let height be 6.
Let area be width multiplied by height.
Print area.
```

## Missing variables

The VM's variable load path uses `nothing` when a name is not found rather than immediately raising an undefined-variable error. Code should therefore avoid relying on accidental missing names.

## Scope

Function calls create a new scope frame for their parameters and local variables. See [Functions](functions.md) and [Architecture](architecture.md).
