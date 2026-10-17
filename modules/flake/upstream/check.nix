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
        name = "upstream-test";
        runtimeInputs = [
          self'.packages.upstream
          pkgs.git
          pkgs.gnugrep
          pkgs.coreutils
        ];
        text = builtins.readFile ./upstream-test.sh;
      };
    in
    {
      checks = lib.optionalAttrs (system == "x86_64-linux") {
        upstream = pkgs.runCommand "upstream" { } ''
          cd "$TMPDIR"
          ${lib.getExe test}
          touch $out
        '';
      };
    };
}
