# sbsearch

[![AI Ready](https://img.shields.io/badge/AI--Ready-yes-brightgreen?style=flat)](https://github.com/johnpapa/ai-ready)

`sbsearch` is a [Harvester support bundle][1] tool that search for keywords in the
resource logs and displayed them in chronological order. It uses the
[`grep` crate](https://crates.io/crates/grep) for fast searching and the
[`ratatui` crate](https://ratatui.rs/) for terminal user interface.

![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/ihcsim/sbsearch/main.yaml)
![GitHub License](https://img.shields.io/github/license/ihcsim/sbsearch)
![GitHub Created At](https://img.shields.io/github/created-at/ihcsim/sbsearch)
![GitHub Tag](https://img.shields.io/github/v/tag/ihcsim/sbsearch)

![screenshot of the sbsearch tui displaying resource logs output](./img/tui.png)

`sbsearch` searches the `logs/` and `nodes/**/logs` folders in the support bundle
for the keyword. Entries from the `logs/` folder are considered workload logs,
while entries from the `nodes/**/logs` are system logs.

📝 System logs without timestamp or don't follow the RFC 3339 timestamp format are
displayed at the end of the log list.

## Usage

To see general usage:

```sh
sbsearch -h
```

```sh
Usage: sbsearch --support-bundle-path <SUPPORT_BUNDLE_PATH> --keyword <KEYWORD>

Options:
  -s, --support-bundle-path <SUPPORT_BUNDLE_PATH>
  -k, --keyword <KEYWORD>
  -h, --help                                       Print help
  -V, --version                                    Print version
```

For example, to search for logs relevant to the PVC
`pvc-tg13d9d2-f7g3-46t1-770d-13wa01c36f01` in the support bundle located at
`~/Downloads/supportbundle_5t66d62c-u8a4-4311-8426-1d8493b2b576_2024-10-17T18-38-27Z`:

```sh
sbsearch \
  -s ~/Downloads/supportbundle_5t66d62c-u8a4-4311-8426-1d8493b2b576_2024-10-17T18-38-27Z \
  -r pvc-tg13d9d2-f7g3-46t1-770d-13wa01c36f01
```

Unarchive the support bundle before passing its path to `sbsearch`.

## Keymaps

### Line Navigation

Keys               | Actions
-------------------| -------
Up/Down arrow keys | Move up/down by one line
`d`                  | Jump down by 25 lines
`u`                  | Jump up by 25 lines
`g`                  | Go to the beginning of the log
`G`                  | Go to the end of the log

### Page Navigation

Keys                 | Actions
---------------------| -------
Left/Right arrow keys| Move left/right by one page
`0`                    | Go to the first page
`9`                    | Go to the last page

### Search

Keys | Actions
-----| -------
`/`    | Enter search mode
Enter| Execute search
`c`    | Clear search

### Others

Keys   | Actions
-------| -------
`<tab>`  | Toggle between workload and system logs
`s`      | Save the current filtered logs to a file
`q`      | Quit the program

## Color Scheme

`sbsearch` uses the following color scheme to highlight different line context:

* warning in yellow
* error in red
* currently selected line in light magenta
* search matches in blue

## Development

To compile the code:

```sh
make check
```

To run unit tests:

```sh
make test
```

To run the program in debug mode:

```sh
make run SUPPORT_BUNDLE_PATH=<path_to_support_bundle> KEYWORD=<keyword>
```

To build the release:

```sh
make release
```

## Contributing

1. Fork the repository and create a branch from `main`
2. Make your changes — run `make check` and `make test` before pushing
3. Update `README.md` if you change keymaps, CLI arguments, or visible behavior
4. Add an entry to `CHANGELOG.md` under `[Unreleased]`
5. Open a pull request against `main` — the CI workflow will run automatically

See [`AGENTS.md`](AGENTS.md) for a full guide to the codebase, build commands, and the maintenance
matrix that lists which files to update together.

## License

See [License](LICENSE).

[1]: https://docs.harvesterhci.io/v1.8/troubleshooting/harvester/#generate-a-support-bundle
