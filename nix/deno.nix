# Deno checks for the test suite fixtures.
{root, ...}: {
  perSystem = {pkgs, ...}: let
    fixtures = root + /chiplab-cli/tests/fixtures;

    src = pkgs.lib.fileset.toSource {
      root = fixtures;
      fileset = pkgs.lib.fileset.unions [
        (fixtures + /deno.json)
        (pkgs.lib.fileset.fileFilter (file: file.hasExt "ts") fixtures)
      ];
    };

    denoCheck = name: command:
      pkgs.stdenvNoCC.mkDerivation {
        name = "fixtures-${name}";
        inherit src;
        nativeBuildInputs = [pkgs.deno];
        buildPhase = ''
          # Deno writes its cache under `$HOME` by default, which is unwritable in the sandbox.
          export DENO_DIR="$NIX_BUILD_TOP/deno-cache"
          ${command}
        '';
        installPhase = "touch $out";
      };
  in {
    make-shells.default.packages = [pkgs.deno];

    jj-fix.tools.deno = {
      command = ["deno" "fmt" "--ext=ts" "-"];
      patterns = ["glob:chiplab-cli/tests/fixtures/*.ts"];
    };

    checks = {
      fixtures-deno-lint = denoCheck "deno-lint" "deno lint";
      fixtures-deno-fmt = denoCheck "deno-fmt" "deno fmt --check";
    };
  };
}
