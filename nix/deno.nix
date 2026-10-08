{
  perSystem = {pkgs, ...}: {
    make-shells.default.packages = [pkgs.deno];
  };
}
