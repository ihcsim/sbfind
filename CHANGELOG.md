# Changelog

All notable changes to sbsearch are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.4] — 2026-02-27

### Added

- Jump navigation keys: `d` (down 25 lines), `u` (up 25 lines)

## [0.0.2] — 2026-01-30

### Added

- Split workload and system logs into separate views toggled by `Tab`
- System logs (from `nodes/**/logs/`) shown in a dedicated view

## [0.0.1] — 2026-01-20

### Added

- Initial release
- Keyword search across Harvester support bundle log files using the `grep` crate
- Chronological log display with RFC 3339 timestamp parsing
- Ratatui TUI with line and page navigation
- Color highlighting: warnings (yellow), errors (red), selected line (light magenta), matches (blue)
- Save filtered results to a timestamped `.log` file with `s`
- Debug logging to `.sbsearch.log` via `--log-level`

[Unreleased]: https://github.com/ihcsim/sbsearch/compare/0.0.4...HEAD
[0.0.4]: https://github.com/ihcsim/sbsearch/compare/0.0.2...0.0.4
[0.0.2]: https://github.com/ihcsim/sbsearch/compare/0.0.1...0.0.2
[0.0.1]: https://github.com/ihcsim/sbsearch/releases/tag/0.0.1
