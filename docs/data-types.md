# Data Types

ENG v0.1.0 has six runtime value categories.

## Number

Numbers are stored as `f64`, so integer-looking and fractional values use the same runtime type:

```engling
Let whole be 5.
Let fraction be 2.5.
Print whole plus fraction.
```

## String

```engling
Let name be "ENG".
Print name.
```

Strings can be concatenated with `plus`.

## Boolean

The literals are:

```engling
true
false
```

Example:

```engling
Let enabled be true.
Print enabled.
```

## List

Lists are created with:

```engling
Make a list called numbers.
```

Values can then be added with `Add`.

## Function

A function definition creates a callable runtime value:

```engling
Define a function called square that takes x and returns x multiplied by x.
```

## Nothing

`nothing` is the runtime value used when a function has no explicit return expression and for missing variable loads in the current VM implementation.

## Truthiness

Conditions use runtime truthiness:

- `false` is false.
- `nothing` is false.
- Number `0` is false; other numbers are true.
- An empty string is false; non-empty strings are true.
- An empty list is false; non-empty lists are true.
- Functions are true.

This matters for `If` and `While`.
