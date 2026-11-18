{ lib, ... }:

{
  helpers.scriptCheck =
    pkgs: name: test:
    lib.optionalAttrs (pkgs.stdenv.hostPlatform.system == "x86_64-linux") {
      ${name} = pkgs.runCommand name { } ''
        cd "$TMPDIR"
        ${lib.getExe test}
        touch $out
      '';
    };
}
