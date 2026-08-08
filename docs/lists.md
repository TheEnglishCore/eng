# Lists

Lists are mutable ordered collections.

## Create a list

```engling
Make a list called scores.
```

A new list starts empty.

## Add values

```engling
Add 10 to scores.
Add 20 to scores.
Add 30 to scores.
```

## Get an item

ENG uses ordinal names:

```engling
Let first_score be Get the first item of scores.
Let second_score be Get the second item of scores.
```

Numeric ordinals are also supported:

```engling
Let value be Get the 1st item of scores.
```

The user-facing ordinals are one-based: `1st` means the first item. Internally, the VM converts the ordinal to a zero-based vector index.

## Set an item

```engling
Set the first item of scores to 5.
Set the 2nd item of scores to 25.
```

The item must already exist; setting an out-of-range item is a runtime error.

## Length

```engling
Print the length of scores.
```

The result is a number.

## Supported ordinal words

The lexer defines:

- first
- second
- third
- fourth
- fifth

Numeric ordinal suffixes such as `1st`, `2nd`, `3rd`, and `4th` are also recognized.

## Out-of-range access

Trying to read or set an item outside the list produces a runtime error explaining the requested user-facing index and the current list size.
