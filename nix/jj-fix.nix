{
  perSystem = {
    config,
    lib,
    pkgs,
    ...
  }: let
    configSetCommands = lib.concatStringsSep "\n" (
      lib.concatLists (
        lib.mapAttrsToList (
          name:
            lib.mapAttrsToList (
              key: value: "jj config set --repo fix.tools.${name}.${key} ${lib.escapeShellArg (builtins.toJSON value)}"
            )
        )
        config.jj-fix.tools
      )
    );

    setup-jj-fix = pkgs.writeShellApplication {
      name = "setup-jj-fix";
      # `$path` in the commands is substituted by `jj fix`, not the shell.
      excludeShellChecks = ["SC2016"];
      text = ''
        echo "This will run the following commands:"
        echo
        cat <<'COMMANDS'
        ${configSetCommands}
        COMMANDS
        echo
        read -rp "Continue? [y/N] " answer
        if [ "$answer" != "y" ] && [ "$answer" != "Y" ]; then
          echo "Aborted."
          exit 1
        fi
        ${configSetCommands}
      '';
    };
  in {
    options = {
      jj-fix.tools = lib.mkOption {
        description = "Formatters to configure as `jj fix` tools, keyed by tool name.";
        default = {};
        type = lib.types.attrsOf (lib.types.submodule {
          options = {
            command = lib.mkOption {
              description = "Command to run, receiving file contents on stdin and writing the formatted contents to stdout.";
              type = lib.types.listOf lib.types.str;
            };

            patterns = lib.mkOption {
              description = "Filesets matching the files this tool applies to.";
              type = lib.types.listOf lib.types.str;
            };
          };
        });
      };
    };

    config = {
      apps.setup-jj-fix = {
        type = "app";
        meta.description = "Configure `jj fix` with the formatters used by the checks.";
        program = lib.getExe setup-jj-fix;
      };
    };
  };
}
