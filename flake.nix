{
  description = "Chiplab CLI";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-26.05-small";

    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };

    make-shell = {
      url = "github:nicknovitski/make-shell";
      inputs.flake-compat.follows = "";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs @ {
    nixpkgs,
    flake-parts,
    make-shell,
    rust-overlay,
    ...
  }:
    flake-parts.lib.mkFlake {inherit inputs;} {
      imports = [make-shell.flakeModules.default];

      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      perSystem = {system, ...}: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [rust-overlay.overlays.default];
        };

        # The overlay is an input because nixpkgs only packages one version of `rustc`, so cannot follow `rust-toolchain.toml`.
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);

        chiplab-cli = rustPlatform.buildRustPackage {
          pname = "chiplab-cli";
          inherit (cargoToml.workspace.package) version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          cargoBuildFlags = ["--package" "chiplab-cli"];
        };
      in {
        _module.args.pkgs = pkgs;

        packages.default = chiplab-cli;

        make-shells.default = {
          packages = [rustToolchain];
        };

        formatter = pkgs.alejandra;

        checks = {
          # `buildRustPackage` runs the workspace tests in its check phase, so this is also the test run.
          package = chiplab-cli;

          clippy = chiplab-cli.overrideAttrs (previous: {
            pname = "${previous.pname}-clippy";
            buildPhase = "cargo clippy --workspace --all-targets -- --deny warnings";
            installPhase = "touch $out";
            doCheck = false;
          });

          fmt =
            pkgs.runCommand "cargo-fmt" {
              nativeBuildInputs = [rustToolchain];
            } ''
              export CARGO_HOME="$TMPDIR/cargo"
              cd ${./.}
              cargo fmt --all --check
              touch $out
            '';

          alejandra =
            pkgs.runCommand "alejandra" {
              nativeBuildInputs = [pkgs.alejandra];
            } ''
              alejandra --check ${./.}
              touch $out
            '';
        };
      };
    };
}
