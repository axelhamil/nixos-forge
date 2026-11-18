{ lib, config, ... }:

let
  inherit (lib) types;

  evalCheck =
    pkgs: name: check:
    let
      failed = lib.attrNames (lib.filterAttrs (_: holds: !holds) check.expect);
    in
    lib.throwIf (failed != [ ]) (lib.concatMapStrings (message: "\n${name}: ${message}") failed) (
      pkgs.writeText name check.text
    );
in
{
  options.evalChecks = lib.mkOption {
    description = "Checks decided at evaluation: each builds a text file once every expectation holds.";
    default = { };
    type = types.attrsOf (
      types.submodule {
        options = {
          text = lib.mkOption {
            description = "Content of the file the check builds.";
            type = types.str;
            default = "";
          };
          expect = lib.mkOption {
            description = "Failure messages, each mapped to a condition that must hold.";
            type = types.attrsOf types.bool;
            default = { };
          };
        };
      }
    );
  };

  config.perSystem =
    { pkgs, system, ... }:
    lib.mkIf (system == "x86_64-linux") {
      checks = lib.mapAttrs (evalCheck pkgs) config.evalChecks;
    };
}
