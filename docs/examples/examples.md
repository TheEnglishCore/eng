# Examples

The repository's `examples/` directory contains small programs designed to exercise individual language features.

## Hello world

```engling
Let greeting be "Hello, world.".
Print greeting.
```

## Arithmetic

```engling
Let a be 20.
Let b be 5.

Print a plus b.
Print a multiplied by b.
Print a divided by b.
Print a modulo b.
```

## Strings

```engling
Print "Hello, " plus "world.".
```

## Conditions

```engling
Let score be 75.

If score is greater than or equal to 60, then
    Print "Pass".
Otherwise
    Print "Fail".
End.
```

## Repeat

```engling
Repeat 3 times
    Print "hi".
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

## Function

```engling
Define a function called square that takes x and returns x multiplied by x.

Print Run square with 5.
```

## List

```engling
Make a list called scores.
Add 10 to scores.
Add 20 to scores.
Set the first item of scores to 5.

Print Get the first item of scores.
Print the length of scores.
```

## Module

`math_helpers.eng`:

```engling
Define a function called square that takes x and returns x multiplied by x.
```

Program:

```engling
Import math_helpers.
Print Run square with 5.
```

## Running examples

From the repository root:

```bash
cargo run -- run examples/01_hello.eng
cargo run -- run examples/02_arithmetic.eng
cargo run -- run examples/18_module_math.eng
```

The repository also includes UI examples that require the `ui` feature.
