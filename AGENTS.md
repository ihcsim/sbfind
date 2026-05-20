# AGENTS.md — sbsearch

## Project Overview

`sbsearch` is a Rust CLI tool that searches [Harvester support bundles](https://docs.harvesterhci.io/v1.8/troubleshooting/harvester/#generate-a-support-bundle)
for keywords and displays matching log entries in chronological order using a terminal user interface.
It uses the [`grep` crate](https://crates.io/crates/grep) for fast searching and
[`ratatui`](https://ratatui.rs/) for the TUI.

Current version: see `Cargo.toml` `[package].version`.

## Repository Structure

```
sbsearch/
├── src/
│   ├── main.rs           # Entry point: CLI arg parsing (clap), logging init, Tui bootstrap
│   ├── sbsearch.rs       # Search engine: Entry, LogType, SearchCache, SBSearch
│   └── tui/
│       ├── mod.rs        # TUI state machine: Tui, Screen, SearchMode, pagination, tests
│       ├── event.rs      # Keyboard event dispatch (crossterm → Tui methods), tests
│       └── render.rs     # Ratatui rendering: Renderer, layout sections
├── testdata/
│   └── support_bundle/   # Sample support bundle used in unit tests
├── .github/
│   └── workflows/
│       ├── main.yaml     # CI: build, test, clippy, fmt
│       └── copilot-setup-steps.yml
├── Cargo.toml            # Dependency manifest (never hardcode versions — read from here)
├── Cargo.lock
├── Makefile              # Developer shortcuts: check, test, run, debug, release, fmt, deps
└── README.md
```

## Tech Stack

| Component | Crate | Purpose |
|-----------|-------|---------|
| CLI parsing | `clap` (derive feature) | `--support-bundle-path`, `--keyword`, `--log-level` |
| TUI rendering | `ratatui` | Terminal layout, list navigation, scrollbar |
| Terminal events | `crossterm` | Keyboard input, raw mode |
| Search | `grep-matcher`, `grep-regex`, `grep-searcher` | Fast regex search over log files |
| Timestamp parsing | `chrono` | RFC 3339 and other timestamp formats → `DateTime<Utc>` |
| Archive support | `zip` | Detect and extract zip support bundles |
| Logging | `log` + `env_logger` | Debug logging to `.sbsearch.log` when `--log-level` is set |
| Search input widget | `tui-input` | Search box with crossterm backend |
| Test temp files | `tempfile` | Temporary file creation in tests |

## Build & Run

```sh
# Check and lint
make check

# Run unit tests
make test

# Run in debug mode (requires an extracted support bundle)
make run SUPPORT_BUNDLE_PATH=<path> KEYWORD=<keyword>
make debug SUPPORT_BUNDLE_PATH=<path> KEYWORD=<keyword>   # LOG_LEVEL=debug

# Build release binary
make release

# Check formatting
make fmt

# Prune unused dependencies
make deps
```

The `--log-level` flag (or `LOG_LEVEL` env var via Makefile) writes debug output to `.sbsearch.log`
in the current directory, not to stdout (which is owned by the TUI).

## Support Bundle Structure

`sbsearch` only searches two directory patterns inside the bundle:

- `logs/` — **workload logs** (`LogType::Workload`)
- `nodes/**/logs/` — **system logs** (`LogType::System`)

Entries without a parseable RFC 3339 timestamp are sorted to the end. The support bundle can be
either a directory or a `.zip` archive — zip detection uses the magic bytes `50 4B 03 04`.

## Key Patterns and Conventions

### State machine (Screen enum)
Screen transitions live in `src/tui/event.rs`. The three screens are:
- `Screen::Main` — main log view
- `Screen::ConfirmExit` — `q` → confirm with `y/n`
- `Screen::ConfirmSave` — `s` → confirm with `y/n`

### Search mode (SearchMode enum)
- `SearchMode::Normal` — navigation keys active
- `SearchMode::Insert` — `/` enters insert mode; `Enter` commits; `Esc` cancels

### Log type toggle
`Tab` toggles between `LogType::Workload` and `LogType::System`. The active log type drives which
slice of `SearchCache` is displayed.

### Pagination
- Page size: `DEFAULT_MAX_ENTRIES_PER_PAGE = 100` (in `src/tui/mod.rs`)
- `page_goto` (1-indexed), `page_final`, `page_reload` flag
- Pagination is applied to workload and system slices independently

### Cache pattern
`SearchCache` is populated once per search term and held for the session lifetime:
```
SearchCache.all              → all matched entries (sorted by timestamp)
SearchCache.workload_entries → LogType::Workload subset
SearchCache.system_entries   → LogType::System subset
```

### Regex matchers in SBSearch
`SBSearch` builds five log-level matchers and two timestamp matchers at construction time. Adding a
new format requires adding a new `RegexMatcher` field and updating `find_log_level` / `find_timestamp`.

## Adding a New Keymap

1. **`src/tui/event.rs`** — add a `KeyCode::Char('x')` arm in the appropriate `Screen`/`SearchMode` match block
2. **`src/tui/mod.rs`** — add the corresponding handler method on `Tui`
3. **`README.md`** — add the key to the relevant keymap table
4. **`src/tui/event.rs` tests** — add a test case in `handle_key_events_on_main_screen` or the relevant test fn

## Adding a New Screen

1. **`src/tui/mod.rs`** — add variant to `Screen` enum
2. **`src/tui/event.rs`** — add a match arm for the new `Screen` variant
3. **`src/tui/render.rs`** — add a render method
4. **`src/tui/mod.rs` `Tui::run()`** — add the new screen to the `terminal.draw()` match
5. Write tests in `src/tui/event.rs`

## CI/CD

CI runs on every push and PR to `main` via `.github/workflows/main.yaml`:
- `cargo build --verbose`
- `cargo test --verbose`
- `cargo clippy -- -D warnings` (warnings are errors)
- `cargo fmt -- --check`

All four checks must pass before merging. Run `make check` and `make test` locally before pushing.

## Common Pitfalls

- **Don't write to stdout** — the TUI owns the terminal. Use `log::info!()` with `--log-level` for
  debug output; it writes to `.sbsearch.log`.
- **Support bundle must be extracted** — the tool does NOT auto-extract zips at runtime (zip detection
  is a helper, not the main path). Pass the path to the extracted directory.
- **Clippy is enforced as errors** — `cargo clippy -- -D warnings` runs in CI. Run `make check`
  before pushing.
- **Page size affects test assertions** — `DEFAULT_MAX_ENTRIES_PER_PAGE` is referenced in
  `src/tui/mod.rs` tests. Update test expectations if you change the constant.
- **rke2.yaml in repo root** — this file contains kubeconfig credentials and should not be committed.
  Add it to `.gitignore` if present.

## Documentation

User-facing documentation lives in `README.md` (usage, keymaps, color scheme, development workflow).
There is no separate docs site. When changing behavior, update `README.md` in the same PR.
