{ lib, ... }:

{
  helpers.wrapForge =
    pkgs: forge:
    {
      commands ? [ ],
      runtimeInputs ? [ ],
      environment ? { },
    }:
    let
      wrapperArgs =
        lib.optionals (commands != [ ]) [
          "--prefix"
          "FORGE_COMMANDS_PATH"
          ":"
          (lib.makeBinPath commands)
        ]
        ++ lib.optionals (runtimeInputs != [ ]) [
          "--prefix"
          "PATH"
          ":"
          (lib.makeBinPath runtimeInputs)
        ]
        ++ lib.concatLists (
          lib.mapAttrsToList (name: value: [
            "--set-default"
            name
            value
          ]) environment
        );
    in
    if wrapperArgs == [ ] then
      forge
    else
      pkgs.symlinkJoin {
        name = "forge";
        paths = [ forge ];
        nativeBuildInputs = [ pkgs.makeBinaryWrapper ];
        postBuild = ''
          wrapProgram $out/bin/forge ${lib.escapeShellArgs wrapperArgs}
        '';
        meta.mainProgram = "forge";
      };
}
