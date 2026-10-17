{ lib, ... }:

{
  perSystem =
    { pkgs, system, ... }:
    let
      fakeNix = pkgs.writeShellApplication {
        name = "nix";
        runtimeInputs = [ pkgs.coreutils ];
        text = builtins.readFile ./reproducibility-fake-nix.sh;
      };

      reproducibility = import ./_reproducibility.nix {
        inherit pkgs;
        nix = fakeNix;
      };

      test = pkgs.writeShellApplication {
        name = "reproducibility-test";
        runtimeInputs = [
          pkgs.coreutils
          pkgs.gnugrep
        ];
        runtimeEnv.REPRODUCIBILITY = lib.getExe reproducibility;
        excludeShellChecks = [ "SC2016" ];
        text = builtins.readFile ./reproducibility-test.sh;
      };
    in
    {
      checks = lib.optionalAttrs (system == "x86_64-linux") {
        reproducibility = pkgs.runCommand "reproducibility" { } ''
          cd "$TMPDIR"
          ${lib.getExe test}
          touch $out
        '';
      };
    };
}
