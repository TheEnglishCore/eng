# Operators

ENG spells operators as English phrases.

## Arithmetic

| ENG | Meaning |
|---|---|
| `plus` | addition |
| `minus` | subtraction |
| `multiplied by` | multiplication |
| `divided by` | division |
| `modulo` | remainder |

Example:

```engling
Let a be 20.
Let b be 5.

Print a plus b.
Print a multiplied by b.
Print a divided by b.
Print a modulo b.
```

Output:

```text
25
100
4
0
```

`plus` also concatenates two strings:

```engling
Print "Hello, " plus "world.".
```

Adding incompatible value types produces a runtime error.

## Equality

```engling
x is equal to y
x is not equal to y
```

Equality supports the same runtime categories where the VM can compare them, including numbers, booleans, strings, and `nothing`.

## Numeric comparisons

```engling
x is greater than y
x is less than y
x is greater than or equal to y
x is less than or equal to y
```

These comparisons require numbers.

## Boolean operators

```engling
condition1 and condition2
condition1 or condition2
```

Example:

```engling
Let age be 25.
Let has_id be true.

If age is greater than 18 and has_id, then
    Print "Allowed".
End.
```

## Precedence

Expressions are parsed in this order, from tighter to looser binding:

1. Multiplication/division/modulo
2. Addition/subtraction
3. Comparisons
4. `and`
5. `or`

Use intermediate variables when an expression would otherwise become difficult to read.
