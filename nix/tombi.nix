{root, ...}: {
  perSystem = {pkgs, ...}: let
    tomlSource = pkgs.lib.fileset.toSource {
      inherit root;
      fileset = pkgs.lib.fileset.fileFilter (file: file.hasExt "toml") root;
    };
  in {
    make-shells.default = {
      packages = [pkgs.tombi];
      env = {
        # Default the devshell to behave the same as the nix check: no auto-fetching of uncached schemas.
        TOMBI_OFFLINE = "true";
      };
    };

    jj-fix.tools.tombi = {
      command = ["tombi" "format" "--quiet" "--offline" "--stdin-filename=$path" "-"];
      patterns = ["glob:**/*.toml"];
    };

    checks = {
      tombi-format = pkgs.stdenvNoCC.mkDerivation {
        name = "tombi-format";
        src = tomlSource;
        nativeBuildInputs = [pkgs.tombi];
        buildPhase = "tombi format --check --offline --verbose";
        installPhase = "touch $out";
      };

      tombi-lint = pkgs.stdenvNoCC.mkDerivation {
        name = "tombi-lint";
        src = tomlSource;
        nativeBuildInputs = [pkgs.tombi];
        buildPhase = "tombi lint --offline --verbose";
        installPhase = "touch $out";
      };
    };
  };
}
