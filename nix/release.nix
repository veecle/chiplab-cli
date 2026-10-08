# The release binaries must run without Nix, but a normal Nix build may link against libraries in `/nix/store`:
#   - Linux: a separate build, linked fully statically against musl, so it runs on any Linux.
#   - macOS: the normal build, which only links system libraries, checked to stay that way.
# See `.github/workflows/release.yaml`.
{root, ...}: {
  perSystem = {
    self',
    lib,
    pkgs,
    chiplabCliArgs,
    ...
  }: let
    # The toolchain runs on the build platform, with the standard library added for the musl target.
    staticRustToolchain = (pkgs.pkgsStatic.pkgsBuildHost.rust-bin.fromRustupToolchainFile (root + /rust-toolchain.toml)).override {
      targets = [pkgs.pkgsStatic.stdenv.hostPlatform.rust.rustcTarget];
    };

    chiplab-cli-static =
      (pkgs.pkgsStatic.makeRustPlatform {
        cargo = staticRustToolchain;
        rustc = staticRustToolchain;
      }).buildRustPackage (chiplabCliArgs
        // {
          # The tests already run in the normal build.
          doCheck = false;

          doInstallCheck = true;
          installCheckPhase = ''
            for bin in $out/bin/*; do
              # Captured first, so a failure of `readelf` itself fails the build instead of passing the `grep`.
              headers=$($READELF --program-headers --dynamic "$bin")
              if grep --extended-regexp 'INTERP|NEEDED' <<<"$headers"; then
                echo "$bin needs a dynamic loader or shared libraries" >&2
                exit 1
              fi
              "$bin" --version
            done
          '';
        });
  in {
    packages = lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
      static = chiplab-cli-static;
    };

    checks =
      lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
        # Builds the release binary on every change, so a broken static build is caught before release time.
        package-static = chiplab-cli-static;
      }
      // lib.optionalAttrs pkgs.stdenv.hostPlatform.isDarwin {
        package-portable = pkgs.runCommand "chiplab-cli-portable" {nativeBuildInputs = [pkgs.cctools];} ''
          for bin in ${self'.packages.default}/bin/*; do
            libraries=$(otool -L "$bin")
            # The first line is the binary's own path, the linked libraries follow indented.
            if grep '^[[:space:]]\+/nix/store' <<<"$libraries"; then
              echo "$bin links libraries from /nix/store" >&2
              exit 1
            fi
          done
          touch $out
        '';
      };
  };
}
