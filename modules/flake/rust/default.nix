{ inputs, lib, ... }:

let
  root = ../../../crates;
in
{
  perSystem =
    { pkgs, system, ... }:
    let
      craneLib = inputs.crane.mkLib pkgs;

      src = lib.fileset.toSource {
        inherit root;
        fileset = lib.fileset.unions [
          (craneLib.fileset.commonCargoSources root)
          (lib.fileset.fileFilter (file: file.hasExt "snap") root)
          (root + "/deny.toml")
        ];
      };

      commonArgs = {
        inherit src;
        pname = "forge";
        version = "0.0.0";
        strictDeps = true;
      };

      cargoArtifacts = craneLib.buildDepsOnly commonArgs;
    in
    {
      packages.forge = craneLib.buildPackage (
        commonArgs
        // {
          inherit cargoArtifacts;
          doCheck = false;
          meta.mainProgram = "forge";
        }
      );

      checks = lib.optionalAttrs (system == "x86_64-linux") {
        rust-clippy = craneLib.cargoClippy (
          commonArgs
          // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          }
        );

        rust-fmt = craneLib.cargoFmt { inherit (commonArgs) src pname version; };

        rust-deny = craneLib.cargoDeny { inherit (commonArgs) src pname version; };

        rust-audit = craneLib.cargoAudit {
          inherit (commonArgs) src pname version;
          inherit (inputs) advisory-db;
        };

        rust-tests = craneLib.cargoNextest (
          commonArgs
          // {
            inherit cargoArtifacts;
            INSTA_UPDATE = "no";
          }
        );
      };
    };
}
