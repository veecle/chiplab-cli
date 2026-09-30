# chiplab-cli

Command-line client for [Chiplab](https://chiplab.veecle.io).

The intended use case is hooking Chiplab into a CI pipeline, so the CLI is the stable interface to interact with it.

## Development

```sh
nix develop
cargo run
```

`rust-toolchain.toml` pins the compiler, so `cargo` works the same with a plain `rustup` install and no Nix.
