# chiplab-cli

Command-line client for [Chiplab](https://chiplab.veecle.ai).

The intended use case is hooking Chiplab into a CI pipeline, so the CLI is the stable interface to interact with it.

## Development

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
