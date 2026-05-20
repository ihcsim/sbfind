## Description

<!-- What does this PR do? Why is this change needed? -->

## Changes

<!-- List the key changes made -->

-

## How to test

<!-- Steps to verify this works. Include the support bundle path and keyword if applicable. -->

1.
2.

## Checklist

- [ ] `make check` passes (cargo check + clippy with `-D warnings`)
- [ ] `make test` passes
- [ ] `make fmt` passes
- [ ] `README.md` updated (if keymaps, CLI args, or behavior changed)
- [ ] `CHANGELOG.md` updated under `[Unreleased]`

## Maintenance matrix

<!-- Check any boxes that apply — and confirm the linked files were updated -->

- [ ] Added/changed key binding → updated `README.md` Keymaps section
- [ ] Changed `Screen` enum → updated `event.rs`, `render.rs`, and `Tui::run()` match
- [ ] Changed `LogType` enum → updated `toggle_log_type()`, `focus_entries()`, `focus_entries_total()`
- [ ] Changed `Args` struct → updated `Makefile` and `README.md` Usage section
- [ ] Changed `DEFAULT_MAX_ENTRIES_PER_PAGE` → updated test assertions in `tui/mod.rs`
- [ ] Changed color scheme → updated `README.md` Color Scheme section
