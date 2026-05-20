# Copilot Instructions — sbsearch

## Language: Rust

- Edition: see `Cargo.toml` `edition` field
- Use `?` for error propagation — avoid `.unwrap()` in non-test code
- Prefer `Box<dyn Error>` as the return error type for functions that can fail with multiple error kinds
- Use `log::info!()` / `log::debug!()` for diagnostics — never `println!()` in library or TUI code (the TUI owns stdout)
- Clippy warnings are treated as errors in CI (`-D warnings`). Fix all clippy lints before committing.
- Run `cargo fmt` before committing. The CI `cargo fmt -- --check` will fail on unformatted code.

## Naming Conventions

- Enum variants: `PascalCase` (e.g., `LogType::Workload`, `Screen::ConfirmExit`)
- Structs: `PascalCase` (e.g., `SearchCache`, `SBSearch`)
- Functions and methods: `snake_case`
- Private state machine helpers on `Tui`: prefix with the action (`nav_`, `page_`, `draw_`, `toggle_`)

## TUI Conventions

- All keyboard handling lives in `src/tui/event.rs` — do not inline key dispatch in `mod.rs`
- All ratatui rendering lives in `src/tui/render.rs` — do not render in `mod.rs` or `event.rs`
- State mutations live in `src/tui/mod.rs` as methods on `Tui`
- The `Screen` enum drives the top-level render dispatch in `Tui::run()`
- The `SearchMode` enum is only relevant within `Screen::Main`

## Test Conventions

- Unit tests live in the same file as the code under test (`#[cfg(test)] mod tests { ... }`)
- Test functions use `snake_case` names prefixed with `test_`
- Use `testdata/support_bundle` for integration-style tests that need real files
- Use `tempfile::NamedTempFile` for tests that write to disk
- Do not add new test dependencies without updating `Cargo.toml`

## Maintenance Matrix

When you change a file, also update these related files:

| If you change… | Also update… |
|----------------|-------------|
| `src/tui/event.rs` key bindings | `README.md` Keymaps section |
| `src/tui/mod.rs` `Screen` enum | `src/tui/event.rs` (add match arm), `src/tui/render.rs` (add render), `src/tui/mod.rs` `Tui::run()` match |
| `src/tui/mod.rs` `SearchMode` enum | `src/tui/event.rs` `SearchMode::*` match arms |
| `src/tui/mod.rs` `DEFAULT_MAX_ENTRIES_PER_PAGE` | Tests in `src/tui/mod.rs` that assert entry counts |
| `src/sbsearch.rs` `LogType` enum | `src/tui/mod.rs` `toggle_log_type()`, `focus_entries()`, `focus_entries_total()` |
| `src/sbsearch.rs` `SBSearch` regex matchers | `src/sbsearch.rs` `find_log_level()` / `find_timestamp()` |
| `src/sbsearch.rs` search directory logic (`is_log_dir`) | `README.md` description, `AGENTS.md` Support Bundle Structure |
| `src/main.rs` `Args` struct | `Makefile` `run`/`debug` targets, `README.md` Usage section |
| `Cargo.toml` dependencies | `AGENTS.md` Tech Stack table |
| Color scheme in `src/tui/render.rs` | `README.md` Color Scheme section |
| `CHANGELOG.md` | Keep a Changelog format — add entry under `[Unreleased]` on every PR, move to versioned section on release |

## Code Style Notes

- Formatting is enforced by `rustfmt` (default config). Run `cargo fmt` before pushing.
- Clippy profile: default `stable` with `-D warnings`. Do not add `#[allow(...)]` without a comment explaining why.
- Unsafe blocks are acceptable only where required by the `grep-searcher` mmap API (see `SBSearch::new`). Add a `// SAFETY:` comment.

## Security Best Practices

**Memory safety**

- Prefer safe Rust — avoid `unsafe` except where required by external APIs (currently only `grep_searcher::MmapChoice::auto()` in `SBSearch::new`). Every `unsafe` block must have a `// SAFETY:` comment explaining the invariant being upheld.
- Do not use raw pointer arithmetic or `std::mem::transmute` without a compelling, documented reason.

**Input validation**

- Treat `--support-bundle-path` as untrusted — validate that the resolved path stays within the intended directory before passing it to filesystem APIs. Reject or sanitize `../` traversal components.
- The `--keyword` argument is compiled into a regex via `RegexMatcher`. Propagate `RegexMatcher::new()` errors with `?` — never `.unwrap()`. Document to users that keywords are treated as regular expressions.

**Error handling**

- Use `?` for error propagation. Avoid `.unwrap()` and `.expect()` outside of tests and startup assertions.
- Do not swallow errors silently — if an error cannot be propagated, log it with `log::warn!()` or `log::error!()` and document why it is non-fatal.

**Secrets and credentials**

- Do not embed credentials in source code or `Cargo.toml`. Use environment variables or a secrets manager.

**Logging**

- Do not log file contents, user-supplied keywords, or file paths that may contain PII or sensitive data.
- `.sbsearch.log` is written to the current working directory when `--log-level` is set — note this in user-facing docs so users don't accidentally log to shared locations.

**Dependencies**

- Run `make deps` (`cargo machete --fix`) to remove unused dependencies before releasing.
- Review new dependencies for known vulnerabilities with `cargo audit` before adding them to `Cargo.toml`.
- Pin dependencies to a minimum version in `Cargo.toml`; rely on `Cargo.lock` for reproducible builds.

**Integer arithmetic**

- Use saturating (`saturating_add`, `saturating_sub`) or checked arithmetic when computing offsets and indices that could overflow (pagination logic in `src/tui/mod.rs` already follows this pattern — maintain it).

## PR Checklist

Before opening a PR:

1. `make check` passes (cargo check + clippy)
2. `make test` passes
3. `make fmt` passes (or run `cargo fmt`)
4. `README.md` updated if behavior or keymaps changed
5. `CHANGELOG.md` updated under `[Unreleased]`
