{
  perSystem =
    { pkgs, self', ... }:
    {
      packages.upstream = pkgs.writeShellApplication {
        name = "upstream";
        runtimeInputs = [
          self'.packages.forge
          pkgs.git
        ];
        text = ''exec forge upstream "$@"'';
      };
    };
}
