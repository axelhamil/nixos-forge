{ lib, self, ... }:

let
  runScripts = [ "flake/hosts/perf-test.py" ];
in
{
  perSystem =
    { pkgs, system, ... }:
    let
      check = "${lib.getExe pkgs.python3} ${./vm-launchers.py}";
    in
    {
      checks = lib.optionalAttrs (system == "x86_64-linux") {
        vm-launchers = pkgs.runCommand "vm-launchers" { } ''
          ${check} ${self}/modules ${lib.escapeShellArgs runScripts}

          mkdir -p bypass
          echo 'machine = create_test_machine(oldmachine=machine, name="booted_machine")' > bypass/test.py
          if ${check} bypass 2> bypass.err; then
            echo "vm-launchers misses a launcher that bypasses machine_command" >&2
            exit 1
          fi
          grep -qF "modules/test.py:1: create_test_machine needs start_command=machine_command" bypass.err

          touch $out
        '';
      };
    };
}
