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
      packages.check-docs = pkgs.writeShellApplication {
        name = "check-docs";
        runtimeInputs = [
          pkgs.lychee
          pkgs.ripgrep
          pkgs.jq
          pkgs.gnugrep
          pkgs.findutils
          pkgs.coreutils
        ];
        runtimeEnv.SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
        text = builtins.readFile ./check-docs.sh;
      };

      checks = lib.optionalAttrs (system == "x86_64-linux") {
        docs = pkgs.runCommand "docs" { } ''
          ${lib.getExe config.packages.check-docs} ${self}
          touch $out
        '';
      };
    };
}
