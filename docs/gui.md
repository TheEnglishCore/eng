# GUI

GUI support is optional and is gated behind the `ui` Cargo feature.

## Build with GUI support

```bash
cargo build --features ui --release
```

Then run a GUI program with:

```bash
eng run examples/25_window_counter.eng --ui
```

## Window

```engling
Make a window called app titled "Counter".
```

## Label

```engling
Add a label to app labeled "Count: 0".
```

## Button

```engling
Add a button to app labeled "Increment".
```

## Text field

The parser also recognizes text fields:

```engling
Add a text field to app labeled "Name".
```

The UI widget declarations are only available in a build with the `ui` feature.

## Button events

```engling
When the Increment button is clicked, run increment.
```

The handler refers to a function by name.

## Updating a label

```engling
Set the label text of count_label to count.
```

The UI bridge converts the ENG value to text before updating the label.

## Why UI is optional

The default interpreter has no GUI dependency. The `ui` feature enables `eframe` and `egui`, keeping the normal interpreter usable in headless environments such as CI and terminal-only environments.
