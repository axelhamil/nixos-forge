{ lib, self, ... }:

{
  perSystem =
    {
      pkgs,
      config,
      system,
      ...
    }:
    {
      packages.init = pkgs.writeShellApplication {
        name = "init";
        runtimeInputs = [
          pkgs.git
          pkgs.gawk
          pkgs.coreutils
        ];
        text = builtins.readFile ./init.sh;
      };

      checks = lib.optionalAttrs (system == "x86_64-linux" && builtins.pathExists ../../hosts/example) {
        init =
          let
            test = pkgs.writeShellApplication {
              name = "init-test";
              runtimeInputs = [
                config.packages.init
                pkgs.git
                pkgs.gnugrep
                pkgs.coreutils
                pkgs.nixfmt
              ];
              runtimeEnv = {
                SOURCE = "${self}";
                NIX_INSTANTIATE = lib.getExe' pkgs.nix "nix-instantiate";
              };
              text = builtins.readFile ./init-test.sh;
            };
          in
          pkgs.runCommand "init" { } ''
            HOME=$TMPDIR ${lib.getExe test}
            touch $out
          '';
      };
    };
}
