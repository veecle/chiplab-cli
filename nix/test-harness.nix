# The fixture test harness, packaged for running the cases outside of `cargo test`, see "Testing" in `CONTRIBUTING.md`.
{
  perSystem = {
    self',
    pkgs,
    ...
  }: let
    test-harness = self'.packages.default.overrideAttrs (previous: {
      pname = "chiplab-cli-test-harness";

      nativeBuildInputs = previous.nativeBuildInputs ++ [pkgs.jq pkgs.makeWrapper];

      # Cargo builds the binary alongside the test, but it goes unused in favour of the default package's, so the cases
      # run against what's shipped rather than a build with the test dependencies' features.
      buildPhase = ''
        runHook preBuild
        cargo test --release --frozen --package chiplab-cli --test cli --no-run \
          --message-format json-render-diagnostics > artifacts.json
        runHook postBuild
      '';

      # The tests already run in the normal build, and the `test-harness` check runs the packaged harness.
      doCheck = false;

      installPhase = ''
        runHook preInstall

        harness=$(jq --raw-output \
          'select(.reason == "compiler-artifact" and .target.kind == ["test"] and .target.name == "cli") | .executable' \
          artifacts.json)
        install -D "$harness" "$out/libexec/chiplab-cli/test-harness"

        makeWrapper "$out/libexec/chiplab-cli/test-harness" "$out/bin/chiplab-cli-test-harness" \
          --set-default TEST_CHIPLAB_CLI ${pkgs.lib.getExe self'.packages.default} \
          --set-default TEST_CHIPLAB_TESTS "$out/share/chiplab-cli/tests"

        mkdir -p "$out/share/chiplab-cli/tests"
        cp -r chiplab-cli/tests/cases chiplab-cli/tests/fixtures "$out/share/chiplab-cli/tests"

        runHook postInstall
      '';

      meta =
        previous.meta
        // {
          description = "Fixture test harness for chiplab-cli";
          mainProgram = "chiplab-cli-test-harness";
        };
    });
  in {
    packages.test-harness = test-harness;

    # Runs the packaged harness over the packaged cases, replaying their exchanges, to check that it works outside of
    # the build tree.
    checks.test-harness =
      pkgs.runCommand "chiplab-cli-test-harness-check" {
        nativeBuildInputs = [pkgs.cacert];
      } ''
        ${pkgs.lib.getExe test-harness}
        touch $out
      '';
  };
}
