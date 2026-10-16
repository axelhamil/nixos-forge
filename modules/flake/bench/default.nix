{
  perSystem =
    { config, pkgs, ... }:
    {
      packages.schbench = pkgs.callPackage ./_schbench.nix { };

      packages.bench = pkgs.writeShellApplication {
        name = "bench";
        runtimeInputs = [
          config.packages.schbench
          pkgs.perf
          pkgs.rt-tests
          pkgs.stress-ng
          pkgs.fio
          pkgs.jq
          pkgs.gawk
          pkgs.coreutils
          pkgs.util-linux
        ];
        text = builtins.readFile ./bench.sh;
      };
    };
}
