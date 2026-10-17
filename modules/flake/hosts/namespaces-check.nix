{
  lib,
  inputs,
  config,
  ...
}:

let
  names = namespace: lib.attrNames config.${namespace};

  known = namespace: lib.concatStringsSep ", " (names namespace);

  wrongHosts = {
    wrong-features = {
      host.features = [ "nvidia" ];
      message = "configurations.nixos.wrong-features.features: unknown name \"nvidia\". Known: ${known "features"}.";
    };

    wrong-profile = {
      host.profile = "ssh";
      message = "configurations.nixos.wrong-profile.profile: unknown name \"ssh\". Known: ${known "profiles"}.";
    };

    wrong-hardware = {
      host.hardware = [ "disk" ];
      message = "configurations.nixos.wrong-hardware.hardware: unknown name \"disk\": neither a module of modules/features/hardware (${known "hardware"}) nor a nixos-hardware profile (https://github.com/NixOS/nixos-hardware).";
    };
  };

  registry =
    pkgs:
    pkgs.writeText "registry.nix" ''
      hostName:
      let
        lib = import ${inputs.nixpkgs}/lib;
        empty = names: lib.genAttrs names (_: { });

        flake = lib.evalModules {
          specialArgs.inputs = {
            nixos-hardware.nixosModules = { };
            nixpkgs.legacyPackages = { };
          };
          modules = [
            ${./options.nix}
            ${./namespaces.nix}
            ${../helpers/default.nix}
            ${../helpers/apps.nix}
            {
              options.systems = lib.mkOption { default = [ "x86_64-linux" ]; };
              options.flake = lib.mkOption {
                default.modules = {
                  nixos.zsh = { };
                  homeManager = { };
                };
              };
              config = {
                profiles = empty (builtins.fromJSON ${lib.strings.escapeNixString (builtins.toJSON (names "profiles"))});
                features = empty (builtins.fromJSON ${lib.strings.escapeNixString (builtins.toJSON (names "features"))});
                hardware = empty (builtins.fromJSON ${lib.strings.escapeNixString (builtins.toJSON (names "hardware"))});
                configurations.nixos = builtins.fromJSON ${
                  lib.strings.escapeNixString (
                    builtins.toJSON (lib.mapAttrs (_: wrong: wrong.host // { stateVersion = "26.05"; }) wrongHosts)
                  )
                };
              };
            }
          ];
        };

        nixos = lib.evalModules {
          specialArgs.pkgs = { };
          modules = [
            flake.config.configurations.nixos.''${hostName}.module
            { _module.check = false; }
          ];
        };
      in
      builtins.attrNames nixos.config
    '';
in
{
  perSystem =
    { pkgs, system, ... }:
    lib.mkIf (system == "x86_64-linux") {
      checks.wrong-namespace =
        pkgs.runCommand "wrong-namespace"
          {
            nativeBuildInputs = [ pkgs.nix ];
            hosts = lib.attrNames wrongHosts;
            messages = lib.mapAttrsToList (_: wrong: wrong.message) wrongHosts;
            __structuredAttrs = true;
          }
          ''
            export NIX_STATE_DIR=$TMPDIR/state NIX_STORE_DIR=$TMPDIR/store NIX_LOG_DIR=$TMPDIR/log NIX_CONF_DIR=$TMPDIR/etc

            for index in "''${!hosts[@]}"; do
              host=''${hosts[$index]}
              if nix-instantiate --eval --strict --expr "import ${registry pkgs} \"$host\"" 2> "$TMPDIR/$host.err"; then
                echo "$host evaluated although its registry names a name of another field"
                exit 1
              fi
              if ! grep -qF -- "''${messages[$index]}" "$TMPDIR/$host.err"; then
                echo "$host did not throw the expected message:"
                echo "  ''${messages[$index]}"
                echo "it threw:"
                cat "$TMPDIR/$host.err"
                exit 1
              fi
              echo "$host: ''${messages[$index]}" >> "$out"
            done
          '';
    };
}
