{
  perSystem = {pkgs, ...}: {
    # For type-checking the test suite fixtures in `chiplab-cli/tests/fixtures/`.
    make-shells.default.packages = [pkgs.deno];
  };
}
