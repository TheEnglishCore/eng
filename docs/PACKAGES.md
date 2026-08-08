# Engling V1 Package Manager

Engling has a small, deliberate V1 package manager. It supports two
sources: **official** packages hosted on the
[eng-packages](https://github.com/TheEnglishCore/eng-packages)
repository, and **community** packages served from any URL the user
points at.

The package manager is intentionally not an npm/Cargo-scale solver. It
does flat name resolution, dependency fetching, SHA-256 verification,
and basic cycle detection, and that's it.

---

## Quick reference

```text
eng install colors                                   # official package
eng install https://github.com/Alice/colors          # community (GitHub)
eng install https://example.com/colors-1.0.0.engpkg  # community (.engpkg)
eng list
eng remove colors
eng search color
eng update
```

`eng --help` lists all of these. `eng search` queries the official
registry only; `eng update` upgrades installed official packages whose
registry version is newer (community packages are left alone unless
their manifest declares `update_url`).

---

## Official vs community

The two flows share the same installer and on-disk layout. They differ
only in *how the archive is found*.

### Official packages

```text
eng install <name>
       │
       ▼
ENGLING_REGISTRY (or the default official URL)
       │
       ▼
registry.json → { name: { version, url, checksum } }
       │
       ▼
fetch <url> (a .engpkg archive hosted on GitHub Releases or anywhere)
       │
       ▼
verify SHA-256 checksum (when provided)
       │
       ▼
extract → ~/.engling/packages/<name>/
```

The default registry URL is:

```text
https://raw.githubusercontent.com/TheEnglishCore/eng-packages/main/registry.json
```

Override it with the `ENGLING_REGISTRY` environment variable. Any
HTTP(S) URL that returns a JSON object keyed by package name works —
useful for mirrors and for testing.

### Community packages

Two flavours are supported:

```text
eng install https://github.com/<owner>/<repo>
eng install https://example.com/<package>-<version>.engpkg
```

GitHub repository URLs are fetched as the codeload zip of the `main`
branch (with `master` as a fallback). The archive is downloaded once
and inspected as if it were a `.engpkg`: a `manifest.json` at the
root is required, the rest of the layout is conventional.

Direct `.engpkg` URLs are downloaded and validated the same way.

The package manager does **not** require the package to be registered
in the official registry. Anyone can publish an Engling package by
hosting a GitHub repo or a single archive URL.

---

## Package format

A V1 package is a ZIP file with the extension `.engpkg`. The
structure is intentionally simple:

```text
colors-1.0.0.engpkg
├── manifest.json
├── README.md
└── src/
    └── colors.eng
```

`manifest.json` is required and must validate against the rules
below. `README.md` and the source tree are optional but conventional.

### `manifest.json`

```json
{
  "name": "colors",
  "version": "1.0.0",
  "description": "Color utilities for Engling",
  "main": "src/colors.eng",
  "license": "MIT",
  "dependencies": ["math"],
  "checksum": "<optional sha256 of the .engpkg, hex>",
  "update_url": "<optional https URL used by `eng update`>"
}
```

Field rules:

| Field          | Required | Notes                                                  |
| -------------- | -------- | ------------------------------------------------------ |
| `name`         | yes      | Lowercase letters/digits/`-`/`_`, starts with a letter, ≤64 chars |
| `version`      | yes      | `major.minor.patch`, all numeric, three components     |
| `main`         | yes      | Path inside the package, no `..` segments               |
| `description`  | no       | Free text, used by `eng search`                        |
| `license`      | no       | SPDX identifier recommended                            |
| `dependencies` | no       | List of other package *names*. V1 has no version specs |
| `checksum`     | no       | SHA-256 of the archive, hex                            |
| `update_url`   | no       | Used by `eng update` for community-installed packages  |

Any name referenced in `dependencies` is itself validated by the same
rules.

---

## Where packages live

Packages land under a single user-level directory, resolved in this
order:

1. `ENGLING_PACKAGES_DIR` environment variable (if set and non-empty).
2. `$HOME/.engling/packages` (works on Linux, macOS, Termux/Android).
3. `%USERPROFILE%\.engling\packages` on Windows.
4. `./.engling/packages` as a last-resort fallback.

A typical install produces:

```text
~/.engling/packages/
├── manifest_index.json   # bookkeeping for `eng list` and `eng update`
├── colors/
│   ├── manifest.json
│   ├── README.md
│   └── src/
│       └── colors.eng
└── math/
    ├── manifest.json
    └── src/
        └── math.eng
```

There is no global "registry" outside `manifest_index.json`. The
manager is intentionally stateless — deleting the directory is safe.

---

## Module resolution

When an Engling program runs `import <name>.` the loader tries, in
order:

1. **Local module** — `<program_dir>/<name>.eng`.
2. **`ENGLING_PATH`** — every directory listed in `ENGLING_PATH`
   (separated by `:` or `;`), checked for `<name>.eng`.
3. **Installed package** — the user-level package store. The
   package's `manifest.json` is read and `<store>/<name>/<main>` is
   used.
4. **Error** — `Could not find module 'name'`.

A local module always wins. This means a project can shadow an
installed package by placing its own `<name>.eng` next to the entry
file. It also means `eng` never silently overrides your code with a
package of the same name.

---

## Security

Community packages are untrusted input. The installer protects
itself in the following ways:

* **Path traversal** — ZIP entries with `..` segments, absolute paths,
  or Windows drive letters (`C:/foo`) are rejected before any file is
  written.
* **Manifest validation** — `name`, `version`, and `main` are
  validated. A malformed manifest aborts the install and nothing is
  written.
* **Checksum verification** — when the registry entry (or community
  manifest) provides a SHA-256, the archive bytes are hashed and
  compared before extraction.
* **No code execution during install** — only `.eng` files are
  *written*. They are never compiled or run by `eng install`.
* **Bounded extraction** — every extracted file is verified to live
  inside the package's installation directory.

All package errors are returned as `EnglingError::Package(...)` and
printed by `error::report`. They never panic.

---

## Creating a community package

The fastest path:

```text
my-eng-package/
├── manifest.json
├── README.md
└── src/
    └── main.eng
```

1. Pick a package name (lowercase letters/digits/`-`/`_`, ≤64 chars).
2. Write a `manifest.json` with the shape above.
3. Put your Engling source under `src/main.eng` (or wherever `main`
   points).
4. Push the repo to GitHub (or anywhere else).
5. Tell your users: `eng install https://github.com/<you>/my-eng-package`.

If you'd rather distribute a single archive, build a ZIP with the
same layout and rename it `my-eng-package-1.0.0.engpkg`. Then users
install with the direct URL:

```text
eng install https://example.com/my-eng-package-1.0.0.engpkg
```

To support SHA-256 verification, compute the digest of the archive
and add it to the registry entry:

```sh
sha256sum my-eng-package-1.0.0.engpkg
```

…and put the resulting hex string into the registry under `checksum`.

---

## Environment variables

| Variable                  | Purpose                                          |
| ------------------------- | ------------------------------------------------ |
| `ENGLING_REGISTRY`        | Override the official registry URL.              |
| `ENGLING_PACKAGES_DIR`    | Override the package installation directory.     |
| `ENGLING_LOCAL_REGISTRY`  | (CLI convenience) Point at a local mock for tests/offline use. |
| `ENGLING_PATH`            | Colon/semicolon-separated module search path.    |

---

## Testing and reproducibility

The test suite under `tests/package_manager.rs` exercises every flow
above using a `LocalFetcher` and an in-memory ZIP builder. No test
ever touches the live internet. To run:

```sh
cargo test
```

To work on the package manager without touching the network, set
`ENGLING_LOCAL_REGISTRY` to a directory that mirrors the URL layout
(URLs map to `<root>/<host>/<path>`):

```sh
ENGLING_LOCAL_REGISTRY=./fixtures/registry eng install colors
```