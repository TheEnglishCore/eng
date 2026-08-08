# Module Development

A module is simply an `.eng` file that can be loaded by another program.

## Create a module

Create `math_helpers.eng`:

```engling
Define a function called square that takes x and returns x multiplied by x.

Define a function called cube that takes x and returns x multiplied by x multiplied by x.
```

## Use the module

Create `main.eng` in the same directory:

```engling
Import math_helpers.

Print Run square with 5.
Print Run cube with 3.
```

Run:

```bash
eng run main.eng
```

## Selective imports

Instead of importing every global from a module:

```engling
From math_helpers use square.
```

Now `square` is available to the importing program.

## Module isolation

The loader executes the imported source in a separate VM, collects its globals, and copies those values into the importer. This is why module execution does not simply continue inside the caller's current local scope.

## Circular imports

If module A imports module B and B eventually imports A, the loader detects the module already being loaded and reports a circular-import error.

## Search paths

Set `ENGLING_PATH` to add module directories outside the current project.

For example, on a Unix-like shell:

```bash
export ENGLING_PATH="$HOME/.engling/modules"
```

Then a module can be resolved from that directory.
