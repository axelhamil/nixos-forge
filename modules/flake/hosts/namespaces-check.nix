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

    wrong-apps = {
      host.apps = [ "disk" ];
      message = "configurations.nixos.wrong-apps.apps: unknown name \"disk\": neither an app of modules/apps (${known "apps"}) nor a nixpkgs package; any attribute path of pkgs works, such as \"kdePackages.kate\".";
    };

    wrong-users = {
      host.users.firefox = { };
      message = "configurations.nixos.wrong-users.users: unknown person \"firefox\": no modules/users/firefox. Known: ${known "users"}.";
    };
  };

  json = value: "builtins.fromJSON ${lib.strings.escapeNixString (builtins.toJSON value)}";

  registry =
    pkgs:
    pkgs.writeText "registry.nix" ''
      hostName:
      (import ${./_registry.nix} {
        lib = import ${inputs.nixpkgs}/lib;
        names = ${
          json (
            lib.genAttrs [
              "profiles"
              "features"
              "hardware"
              "apps"
              "users"
            ] names
          )
        };
        modules = [
          ${./options.nix}
          ${./namespaces.nix}
          ${./users.nix}
          ${../helpers/default.nix}
          ${../helpers/apps.nix}
          {
            configurations.nixos = ${
              json (lib.mapAttrs (_: wrong: wrong.host // { stateVersion = "26.05"; }) wrongHosts)
            };
          }
        ];
      }).host hostName
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
                echo "$host evaluated although its registry names an unknown name"
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
