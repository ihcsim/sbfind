# Copilot cloud agent instructions for `sbsearch`

## Repository purpose
- `sbsearch` is a Rust CLI/TUI tool for searching Harvester support bundle logs.
- Search logic is in `src/sbsearch.rs`; terminal UI logic is in `src/tui/`.

## Quick orientation
- Entry point: `/home/runner/work/sbsearch/sbsearch/src/main.rs`
- Search/indexing/parsing: `/home/runner/work/sbsearch/sbsearch/src/sbsearch.rs`
- TUI state and navigation: `/home/runner/work/sbsearch/sbsearch/src/tui/mod.rs`
- Key handling: `/home/runner/work/sbsearch/sbsearch/src/tui/event.rs`
- Rendering: `/home/runner/work/sbsearch/sbsearch/src/tui/render.rs`
- Test fixtures: `/home/runner/work/sbsearch/sbsearch/testdata/support_bundle`

## Build, test, and lint
Run from `/home/runner/work/sbsearch/sbsearch`:

```bash
cargo build --verbose
cargo test --verbose
cargo fmt -- --check
cargo clippy -- -D warnings
```

`Makefile` shortcuts:
- `make check` → `cargo check` + `cargo clippy -- -D warnings`
- `make test` → `cargo test -- --nocapture`
- `make fmt` → `cargo fmt -- --check`
- `make run SUPPORT_BUNDLE_PATH=<path> KEYWORD=<keyword>`

## Implementation guidance
- Keep changes focused and small; avoid broad refactors.
- Prefer existing module boundaries:
  - search/parsing behavior in `sbsearch.rs`
  - UI behavior in `tui/mod.rs`, `tui/event.rs`, and `tui/render.rs`
- Preserve existing keyboard mappings unless the task explicitly changes UX behavior.
- Do not modify `testdata/` unless the task explicitly requires fixture updates.
- Optional runtime logging writes to `.sbsearch.log` when `--log-level` is provided.

## Validation guidance
- For search behavior changes, run targeted tests in `src/sbsearch.rs` and full `cargo test`.
- For UI/keymap changes, run tests in `src/tui/event.rs` and `src/tui/mod.rs`, then full `cargo test`.
- Always run `cargo fmt -- --check` before finalizing.

## Errors encountered during onboarding and workarounds
- Encountered error while running `cargo clippy -- -D warnings`:
  - `clippy::unnecessary_min_or_max` in:
    - `/home/runner/work/sbsearch/sbsearch/src/tui/mod.rs:295`
    - `/home/runner/work/sbsearch/sbsearch/src/tui/mod.rs:304`
  - This is pre-existing in the current branch and causes clippy to fail with `-D warnings`.
- Workaround used during onboarding:
  - Use `cargo build --verbose`, `cargo test --verbose`, and `cargo fmt -- --check` to validate unchanged behavior.
  - Treat the clippy failure above as a known pre-existing issue unless the task includes fixing lint failures.
