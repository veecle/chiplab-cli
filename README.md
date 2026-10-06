# chiplab-cli

Command-line client for [Chiplab](https://chiplab.veecle.ai).

The intended use case is hooking Chiplab into a CI pipeline, so the CLI is the stable interface to interact with it.

## Installation

Each [release](https://github.com/veecle/chiplab-cli/releases) provides prebuilt binaries for Linux (x86_64 and aarch64, statically linked) and macOS (aarch64):

```sh
curl -fsSL https://github.com/veecle/chiplab-cli/releases/latest/download/chiplab-cli-x86_64-unknown-linux-musl.tar.gz | tar -xz
./chiplab-cli --help
```

Replace `latest/download` with `download/<tag>` (e.g. `download/v0.1.0`) to pin a version.
The release also contains `sha256sums.txt` to verify the download.

It can also be built from source with `cargo`:

```sh
cargo install --locked --git https://github.com/veecle/chiplab-cli --tag v0.1.0 chiplab-cli
```

Or run with Nix:

```sh
nix run github:veecle/chiplab-cli/v0.1.0 -- --help
```

## Development

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for setting up a development environment and other contribution guidelines.
