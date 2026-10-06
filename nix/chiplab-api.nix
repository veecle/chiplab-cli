{root, ...}: {
  perSystem = {
    pkgs,
    rustToolchain,
    cargoToml,
    ...
  }: let
    # Client for the internal Chiplab API, generated from its OpenAPI spec.
    generated-chiplab-api =
      pkgs.runCommand "generated-chiplab-api" {
        nativeBuildInputs = [
          pkgs.jq
          pkgs.openapi-generator-cli
          pkgs.tombi
          rustToolchain
        ];
      } ''
        # The Rust generator emits discriminated unions as internally tagged enums, which
        # consume the tag before deserializing the variant, so the variants must not also
        # declare it as a field.
        # Upstream issue: <https://github.com/openapitools/openapi-generator/issues/22344>
        jq '
          [ .components.schemas[]
            | select(.discriminator)
            | .discriminator.propertyName as $prop
            | .oneOf[]."$ref"
            | { name: ltrimstr("#/components/schemas/"), $prop }
          ] as $variants
          | reduce $variants[] as $v (.;
              .components.schemas[$v.name] |= (
                del(.properties[$v.prop])
                | if .required then .required -= [$v.prop] else . end
              )
            )
        ' ${root + "/chiplab-api.json"} > spec.json

        openapi-generator-cli generate \
          --input-spec spec.json \
          --generator-name rust \
          --template-dir ${root + "/openapi-templates"} \
          --output $out \
          --global-property apiDocs=false,modelDocs=false \
          --additional-properties packageName=chiplab-api

        # Drop supporting files that are only relevant to a standalone repository.
        rm -r \
          $out/.gitignore \
          $out/.openapi-generator \
          $out/.openapi-generator-ignore \
          $out/.travis.yml \
          $out/README.md \
          $out/git_push.sh

        # Match the formatting applied to committed files.
        find $out -name '*.rs' -exec rustfmt --edition ${cargoToml.workspace.package.edition} {} +
        (cd $out && tombi format --offline --quiet)
      '';

    update-chiplab-api = pkgs.writeShellApplication {
      name = "update-chiplab-api";
      runtimeInputs = [pkgs.coreutils pkgs.git];
      text = ''
        root="$(git rev-parse --show-toplevel)"
        rm -rf "$root/chiplab-api"
        cp -r --no-preserve=mode,ownership ${generated-chiplab-api} "$root/chiplab-api"
        echo "chiplab-api/ updated"
      '';
    };
  in {
    apps.update-chiplab-api = {
      type = "app";
      meta.description = "Regenerate the chiplab-api client from chiplab-api.json.";
      program = pkgs.lib.getExe update-chiplab-api;
    };

    checks.generated-chiplab-api = pkgs.runCommand "generated-chiplab-api-check" {} ''
      if ! diff --color=always -ru ${root + "/chiplab-api"} ${generated-chiplab-api}
      then
        echo
        echo $'\e[31mERROR\e[0m: chiplab-api/ is out of date, run:'
        echo "    nix run .#update-chiplab-api"
        exit 1
      fi
      touch $out
    '';
  };
}
