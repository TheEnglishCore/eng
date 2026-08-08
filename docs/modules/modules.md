# Modules and Imports

ENG modules are `.eng` files. A module is loaded and executed in its own VM; its resulting global bindings are made available to the importing program.

## Import everything

Suppose `math_helpers.eng` contains:

```engling
Define a function called square that takes x and returns x multiplied by x.
Define a function called double that takes x and returns x multiplied by 2.
```

Then another file can use:

```engling
Import math_helpers.

Print Run square with 5.
Print Run double with 5.
```

The module name is resolved as `math_helpers.eng`.

## Selective import

Import named bindings:

```engling
From math_helpers use square.
```

Multiple names are supported:

```engling
From math_helpers use square and double.
```

## Resolution

For a module named `math_helpers`, the loader first checks:

```text
<directory containing the importing .eng file>/math_helpers.eng
```

It then checks directories listed in `ENGLING_PATH`.

`ENGLING_PATH` is split using both `;` and `:` by the current implementation, so paths can be supplied in either common style.

## Exports

There is no explicit `export` keyword in v0.1.0. Top-level globals created by the module are collected as its exports.

In practice, top-level function definitions are the normal way to expose reusable functionality.

## Caching and circular imports

Loaded modules are cached. A circular import is detected and reported as a module error instead of recursing indefinitely.

## Missing modules

If the loader cannot find `<name>.eng`, the runtime reports a module error including the path it tried.
