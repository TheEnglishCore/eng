# Engling V1 — Package Manager Progress

Check off each item as it is completed. Items not yet done remain `- [ ]`.
This file lives next to `Cargo.toml` so a fresh session can resume from where
the previous one stopped.

## Phase 0 — Inspect
- [x] Read existing `src/package/*` (mod, manifest, version, registry,
      source, fetcher, store, installer, commands) — substantial scaffold
      already present
- [x] Read existing `src/cli.rs`, `src/runtime.rs`, `src/error.rs`,
      `Cargo.toml`, `README.md`
- [x] Confirm Rust toolchain: rustc 1.96.0 / cargo 1.96.0 — modern,
      no downgrades needed
- [x] Confirm no `docs/` directory exists — created

## Phase 1 — Wire CLI
- [x] Add `install`, `remove`, `list`, `search`, `update` subcommands
      to `src/cli.rs`
- [x] Keep `run` and repl behaviour intact
- [x] Route package errors through `error::report` (no panic on
      expected package failures)
- [x] `ENGLING_LOCAL_REGISTRY` honours a directory mock for offline use

## Phase 2 — Integrate module resolution
- [x] Update `runtime::ModuleLoader::resolve_path` so packages resolve
      after local + ENGLING_PATH lookups
- [x] Document resolution order in `docs/PACKAGES.md`
- [x] Avoid breaking existing imports

## Phase 3 — Harden package flow
- [x] `engpkg` extraction rejects `..`, absolute paths, and entries
      outside the staging dir
- [x] Windows drive letters (`C:/foo`) rejected at sanitize time
- [x] Manifest validation rejects invalid names/versions/main/deps
- [x] Empty-string checksum treated as "no checksum" (registry
      convenience)
- [x] SHA-256 checksum verification works
- [x] Registry wire format is flat (transparent serde) so the
      default-shape JSON serializes identically to the parsed shape
- [x] Error messages never say "panicked"

## Phase 4 — Tests (no internet)
- [x] Registry parsing
- [x] Manifest parsing
- [x] Package name validation
- [x] Version parsing
- [x] Direct URL detection
- [x] GitHub URL handling (including blob/tree rejection)
- [x] Local/mock community repository
- [x] Package installation (registry, direct URL, GitHub URL)
- [x] Package resolution
- [x] Package removal
- [x] Package listing
- [x] Missing package
- [x] Invalid package / invalid checksum / corrupt archive / malicious
      archive
- [x] Dependency cycle detection
- [x] `ENGLING_REGISTRY` env override
- [x] Rebuild-after-corruption
- [x] `tests/package_manager.rs` (38 tests, all green, all serialised
      through `serial_test` to avoid env-var races)

## Phase 5 — Documentation
- [x] `docs/PACKAGES.md` (official vs community, structure, manifest,
      install dir, resolution order, security, ENGLING_REGISTRY,
      creating a community package)
- [x] `README.md` updated to mention the package manager and link to
      `docs/PACKAGES.md`

## Phase 6 — Verification
- [x] `cargo fmt --check`
- [x] `cargo test` — 131 tests across lib, integration, package_manager,
      and probe suites, all pass
- [x] `cargo build --release`
- [x] `eng --help` lists install/remove/list/search/update
- [x] `eng list` (no packages) prints "No Engling packages are
      installed."
- [x] `eng search colors` against the local mock returns the entry
- [x] `eng install colors` succeeds and is recorded in the index
- [x] `eng install math` succeeds
- [x] `eng install https://github.com/Bob/demo` (community) succeeds
      with the mock codeload layout
- [x] `eng run /tmp/use_colors.eng` resolves `import colors.` and
      prints the package's output
- [x] `eng remove colors` works and updates the list
- [x] `eng update` reports no updates when current
- [x] `eng install does-not-exist` returns a clean structured error,
      exit code 1, no panic
- [x] `eng remove missing` returns a clean structured error

## Phase 7 — Release ZIP
- [x] `cargo build --release`
- [x] `engling-v1-package-manager.zip` built at `/root/engling-v1-package-manager.zip`
      (≈1.2 MB; contains source, tests, examples, docs, README,
      Cargo.toml, Cargo.lock, PROGRESS.md, and the release binary)
- [x] Excludes `target/` build directory

## Final status

| Item                       | Result                                       |
| -------------------------- | -------------------------------------------- |
| Package manager            | COMPLETE                                     |
| Official registry URL      | https://raw.githubusercontent.com/TheEnglishCore/eng-packages/main/registry.json |
| Official install command   | `eng install <name>`                         |
| Community install command  | `eng install <URL>`                          |
| Tests                      | PASS — 131/131 (lib 49 + integration 43 + package_manager 38 + probe 1) |
| Build                      | PASS — `cargo build --release`               |
| `cargo fmt --check`        | PASS                                         |
| ZIP                        | `/root/engling-v1-package-manager.zip`       |
