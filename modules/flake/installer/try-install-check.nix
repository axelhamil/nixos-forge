{ lib, ... }:

{
  perSystem =
    {
      pkgs,
      system,
      self',
      ...
    }:
    let
      test = pkgs.writeShellApplication {
        name = "try-install-test";
        runtimeInputs = [
          self'.packages.try-install
          pkgs.gnugrep
        ];
        text = builtins.readFile ./try-install-test.sh;
      };
    in
    {
      checks = lib.optionalAttrs (system == "x86_64-linux") {
        try-install = pkgs.runCommand "try-install" { } ''
          cd "$TMPDIR"
          ${lib.getExe test}
          touch $out
        '';
      };
    };
}
