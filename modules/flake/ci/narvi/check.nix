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
      fakeGh = pkgs.writeShellApplication {
        name = "gh";
        runtimeInputs = [ pkgs.coreutils ];
        text = builtins.readFile ./fake-gh.sh;
      };

      narviWithFakeGh = import ./_package.nix {
        inherit pkgs;
        gh = fakeGh;
      };

      test = pkgs.writeShellApplication {
        name = "narvi-test";
        runtimeInputs = [
          pkgs.python3
          pkgs.jq
          pkgs.coreutils
          pkgs.gnugrep
        ];
        runtimeEnv = {
          NARVI = lib.getExe self'.packages.narvi;
          NARVI_FAKE_GH = lib.getExe narviWithFakeGh;
          STUB = ./stub.py;
          MESSAGES = ./messages.json;
        };
        excludeShellChecks = [ "SC2016" ];
        text = builtins.readFile ./narvi-test.sh;
      };
    in
    {
      checks = lib.optionalAttrs (system == "x86_64-linux") {
        narvi = pkgs.runCommand "narvi" { } ''
          cd "$TMPDIR"
          ${lib.getExe test}
          touch $out
        '';
      };
    };
}
