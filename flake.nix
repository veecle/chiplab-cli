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

    veecle-utilities = {
      url = "github:veecle/utilities";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-parts.follows = "flake-parts";
      inputs.make-shell.follows = "make-shell";
    };
  };

  outputs = inputs @ {
    nixpkgs,
    flake-parts,
    make-shell,
    rust-overlay,
    veecle-utilities,
    ...
  }:
    flake-parts.lib.mkFlake {inherit inputs;} {
      imports = [
        make-shell.flakeModules.default
        veecle-utilities.flakeModules.default
        ./nix/chiplab-api.nix
        ./nix/deno.nix
        ./nix/release.nix
        ./nix/rust.nix
        ./nix/test-harness.nix
        ./nix/tombi.nix
      ];

      # Instead of each module having to use some number of `../`'s to reference the repository root,
      # pass down a nix path they can construct sources from.
      _module.args.root = ./.;

      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      perSystem = {system, ...}: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            # The overlay is used because nixpkgs only packages one version of `rustc`, so cannot follow `rust-toolchain.toml`.
            rust-overlay.overlays.default
          ];
        };
      in {
        _module.args.pkgs = pkgs;
      };
    };
}
