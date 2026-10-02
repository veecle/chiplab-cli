{root, ...}: {
  perSystem = {pkgs, ...}: let
    rustToolchain = pkgs.rust-bin.fromRustupToolchainFile (root + /rust-toolchain.toml);

    rustPlatform = pkgs.makeRustPlatform {
      cargo = rustToolchain;
      rustc = rustToolchain;
    };

    cargoToml = builtins.fromTOML (builtins.readFile (root + /Cargo.toml));

    rustSource = pkgs.lib.fileset.toSource {
      inherit root;
      fileset = pkgs.lib.fileset.unions (
        [
          (root + /Cargo.toml)
          (root + /Cargo.lock)
        ]
        ++ map (member: root + "/${member}") cargoToml.workspace.members
      );
    };

    chiplab-cli = rustPlatform.buildRustPackage {
      pname = "chiplab-cli";
      inherit (cargoToml.workspace.package) version;
      src = rustSource;
      cargoLock.lockFile = root + /Cargo.lock;
      cargoBuildFlags = ["--package" "chiplab-cli"];
    };
  in {
    packages.default = chiplab-cli;

    make-shells.default = {
      packages = [
        rustToolchain
        pkgs.cargo-nextest
      ];
    };

    jj-fix.tools.rustfmt = {
      command = ["rustfmt" "--edition" cargoToml.workspace.package.edition];
      patterns = ["glob:**/*.rs"];
    };

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
          cd ${rustSource}
          cargo fmt --all --check
          touch $out
        '';
    };
  };
}
