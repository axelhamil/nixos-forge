{ config, ... }:

{
  perSystem = config.helpers.baseCheck "forge" {
    machine =
      { pkgs, ... }:
      let
        command =
          name: contract:
          pkgs.writeShellApplication {
            name = "forge-${name}";
            text = ''
              if [ "''${1:-}" = --forge-describe ]; then
                echo '{"contract":${toString contract},"name":"${name}","summary":"Prints its arguments"}'
                exit 0
              fi
              echo "${name} $* contract=$FORGE_CONTRACT"
              exit 3
            '';
          };
      in
      {
        forge.commands = [
          (command "hello" 1)
          (command "future" 2)
        ];
      };
    testScripts = [ ./test.py ];
  };
}
