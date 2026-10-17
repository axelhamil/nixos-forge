{ lib, self, ... }:

let
  lock = lib.importJSON (self + "/flake.lock");

  sourceInfo =
    input:
    let
      info = input.sourceInfo or input;
    in
    {
      path = info.outPath;
    }
    // lib.getAttrs (lib.intersectLists (lib.attrNames info) [
      "rev"
      "shortRev"
      "lastModified"
      "lastModifiedDate"
      "narHash"
      "revCount"
    ]) info;

  sourcesFrom =
    key: input:
    lib.foldl' (
      sources: name:
      let
        ref = lock.nodes.${key}.inputs.${name};
      in
      if builtins.isString ref then sources // sourcesFrom ref input.inputs.${name} else sources
    ) { ${key} = sourceInfo input; } (lib.attrNames (lock.nodes.${key}.inputs or { }));

  sources = removeAttrs (sourcesFrom lock.root self) [ lock.root ];

  forkToplevels =
    pkgs:
    pkgs.writeText "fork-toplevels.nix" ''
      fork:
      builtins.mapAttrs (_: host: host.config.system.build.toplevel.drvPath)
        (import ${./_call-fork.nix} {
          lockFile = fork + "/flake.lock";
          sources = builtins.fromJSON (builtins.readFile ${pkgs.writeText "fork-sources.json" (builtins.toJSON sources)});
          inherit fork;
        }).nixosConfigurations
    '';
in
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
          pkgs.gnused
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
                FORK_TOPLEVELS = "${forkToplevels pkgs}";
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
