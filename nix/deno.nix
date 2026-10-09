# Deno checks for the test suite fixtures.
{root, ...}: {
  perSystem = {pkgs, ...}: let
    fixtures = root + /chiplab-cli/tests/fixtures;

    chiplabDTsUrl = "https://chiplab.veecle.ai/mcp/resources/schemas/chiplab.d.ts";

    src = pkgs.lib.fileset.toSource {
      inherit root;
      fileset = pkgs.lib.fileset.unions [
        (root + /chiplab.d.ts)
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
          cd chiplab-cli/tests/fixtures
          ${command}
        '';
        installPhase = "touch $out";
      };

    update-chiplab-d-ts = pkgs.writeShellApplication {
      name = "update-chiplab-d-ts";
      runtimeInputs = [pkgs.curl pkgs.git];
      text = ''
        root="$(git rev-parse --show-toplevel)"
        curl -fsSL ${chiplabDTsUrl} -o "$root/chiplab.d.ts"
        echo "chiplab.d.ts updated"
      '';
    };
  in {
    make-shells.default.packages = [pkgs.deno update-chiplab-d-ts];

    jj-fix.tools.deno = {
      command = ["deno" "fmt" "--ext=ts" "-"];
      patterns = ["glob:chiplab-cli/tests/fixtures/*.ts"];
    };

    checks = {
      chiplab-d-ts-up-to-date = pkgs.fetchurl {
        url = chiplabDTsUrl;
        sha256 = builtins.hashFile "sha256" (root + /chiplab.d.ts);
      };
      fixtures-deno-lint = denoCheck "deno-lint" "deno lint";
      fixtures-deno-fmt = denoCheck "deno-fmt" "deno fmt --check";
      fixtures-deno-check = denoCheck "deno-check" ''
        shopt -s extglob
        deno check !(does-not-compile).ts
      '';
    };
  };
}
