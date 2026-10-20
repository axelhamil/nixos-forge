{ lib, options, ... }:

let
  hostOptions = options.configurations.nixos.type.getSubOptions [ ];
  personOptions = options.users.type.getSubOptions [ ];
in
{
  perSystem =
    {
      pkgs,
      system,
      ...
    }:
    let
      header = pkgs.writeText "options-header.md" ''
        # Host options

        Fields of `configurations.nixos.<host>`, set in `modules/hosts/<host>/default.nix`. Generated from [`modules/flake/hosts/options.nix`](../../modules/flake/hosts/options.nix) by `nix run .#options-doc`: do not edit by hand.

      '';

      personHeader = pkgs.writeText "person-options-header.md" ''

        # Person options

        Fields of `users.<person>`, set in `modules/users/<person>/default.nix`: the person a host gives an account with `users.<person>`. Generated from [`modules/flake/hosts/users.nix`](../../modules/flake/hosts/users.nix).

      '';

      body =
        fields:
        (pkgs.nixosOptionsDoc {
          options = fields;
          transformOptions =
            option:
            option
            // {
              name = lib.removePrefix "<name>." option.name;
              declarations = [ ];
            };
        }).optionsCommonMark;

      reference = pkgs.runCommand "options.md" { } ''
        cat ${header} ${body hostOptions} ${personHeader} ${body personOptions} > $out
      '';
    in
    {
      packages.options-doc = pkgs.writeShellApplication {
        name = "options-doc";
        runtimeInputs = [ pkgs.coreutils ];
        text = ''
          install -m 644 ${reference} docs/reference/options.md
        '';
      };

      checks = lib.optionalAttrs (system == "x86_64-linux") {
        options-doc = pkgs.runCommand "options-doc" { } ''
          if ! diff -u ${../../../docs/reference/options.md} ${reference}; then
            echo "docs/reference/options.md is stale: run nix run .#options-doc" >&2
            exit 1
          fi
          touch $out
        '';
      };
    };
}
