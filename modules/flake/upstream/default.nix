{
  perSystem =
    { pkgs, ... }:
    {
      packages.upstream = pkgs.writeShellApplication {
        name = "upstream";
        runtimeInputs = [
          pkgs.git
          pkgs.gnugrep
          pkgs.gnused
          pkgs.coreutils
        ];
        text = builtins.readFile ./upstream.sh;
      };
    };
}
