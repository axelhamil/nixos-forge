{ lib, ... }:

{
  perSystem =
    { pkgs, system, ... }:
    let
      fakeNix = pkgs.writeShellApplication {
        name = "nix";
        runtimeInputs = [ pkgs.coreutils ];
        text = ''
          echo "nix $*" >>"$CALLS"
          case "$1" in
            eval)
              [ "$2 $3" = "--json .#ci.kernelPins" ] || exit 1
              cat "$PINS"
              ;;
            path-info)
              [ "$2 $3" = "--store https://forge.cachix.org" ] || exit 1
              [ "$4" != /nix/store/cccccccccccccccccccccccccccccccc-linux-6.18-modules ]
              ;;
            copy)
              [ "$2 $3" = "--from https://forge.cachix.org" ] || exit 1
              [ -z "''${COPY_FAIL:-}" ] || [ "$4" != "$COPY_FAIL" ]
              ;;
            *) exit 1 ;;
          esac
        '';
      };

      fakeCachix = pkgs.writeShellApplication {
        name = "cachix";
        runtimeInputs = [ pkgs.coreutils ];
        text = ''
          echo "cachix $*" >>"$CALLS"
          [ -z "''${CACHIX_FAIL:-}" ] || [ "$3" != "$CACHIX_FAIL" ]
        '';
      };

      ciPinKernels = import ./_ci-pin-kernels.nix {
        inherit pkgs;
        nix = fakeNix;
        cachix = fakeCachix;
      };

      test = pkgs.writeShellApplication {
        name = "pin-kernels-test";
        runtimeInputs = [
          pkgs.coreutils
          pkgs.gnugrep
        ];
        runtimeEnv = {
          PINS = ./_fixtures/kernel-pins.json;
          CI_PIN_KERNELS = lib.getExe ciPinKernels;
        };
        text = builtins.readFile ./pin-kernels-test.sh;
      };
    in
    {
      checks = lib.optionalAttrs (system == "x86_64-linux") {
        ci-pin-kernels = pkgs.runCommand "ci-pin-kernels" { } ''
          cd "$TMPDIR"
          ${lib.getExe test}
          touch $out
        '';
      };
    };
}
