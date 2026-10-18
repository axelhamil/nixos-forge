{
  perSystem =
    { pkgs, self', ... }:
    {
      packages.upstream = pkgs.writeShellApplication {
        name = "upstream";
        runtimeInputs = [
          pkgs.git
          pkgs.gnugrep
          pkgs.gnused
          pkgs.coreutils
          self'.packages.narvi
        ];
        text = builtins.readFile ./upstream.sh;
      };
    };
}
