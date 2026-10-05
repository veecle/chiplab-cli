# Contributing to chiplab-cli

## Development environment

The primary supported development environment is the included nix devshell.

[`nix-direnv`](https://github.com/nix-community/nix-direnv) is recommended for easy setup.
After running `direnv allow` once, the devshell activates automatically when you `cd` into the repository, and `.env` is read for any local customization needed.
Otherwise you can manually enter the devshell and source `.env` each time you enter the repo.

```sh
nix develop
source .env
cargo run -- --help
```

The required dependencies should be minimal, and the toolchain itself is pinned via `rust-toolchain.toml`, so a standard dev environment with a plain `rustup` install should also be usable.
Some additional dev tools (e.g. formatters and linters used by the checks) may be needed; see the devshell and checks in [`flake.nix`](flake.nix) and [`nix/`](nix) for details.

If you use [`jj`](https://jj-vcs.github.io/jj/), `nix run .#setup-jj-fix` configures `jj fix` in the repository with the same formatters the checks enforce.

## Testing

The main test approach is fixture tests in `chiplab-cli/tests/cases/`, which run the real binary and snapshot its output.
Each case is a directory, optionally nested in subdirectories to group related cases, containing:

- `args`: the command-line arguments, one per line.
- `expected.txt`: the expected exit status, stdout and stderr.

To add a case, create a directory with an `args` file, then generate its `expected.txt` by re-snapshotting.
When the output changes intentionally, re-snapshot and review the diff before committing:

```sh
SNAPSHOTS=overwrite cargo test --test cli
```

In `expected.txt`, `[..]` matches any text within a line and `...` on a line by itself matches any number of lines.
See [`snapbox::Assert`](https://docs.rs/snapbox/latest/snapbox/assert/struct.Assert.html) for the full wildcard details.
