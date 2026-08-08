# Strings

Strings are double-quoted text values.

## Creating strings

```engling
Let language be "ENG".
Let message be "Programming in plain English.".
```

## Printing

```engling
Print "Hello, world.".
```

## Concatenation

Use `plus`:

```engling
Let first be "Hello, ".
Let second be "world.".
Let message be first plus second.
Print message.
```

The VM only permits string + string or number + number for `plus`.

This will fail:

```engling
Let text be "value".
Print text plus 5.
```

because the operands have incompatible types.

## Quotes

String literals use double quotes. The current lexer treats a newline before the closing quote as an unterminated string error.

## Empty strings

An empty string is valid:

```engling
Let message be "".
```

It is falsey when used as a condition.
