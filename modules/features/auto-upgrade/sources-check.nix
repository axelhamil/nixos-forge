{
  lib,
  inputs,
  config,
  ...
}:

let
  inherit (config) base features;

  refusals =
    mirrors:
    let
      machine = inputs.nixpkgs.lib.nixosSystem {
        modules = [
          base
          features.auto-upgrade
          {
            nixpkgs.hostPlatform = "x86_64-linux";
            system.stateVersion = "26.05";
            update.flake = "github:alice/nixos";
            update.mirrors = mirrors;
          }
        ];
      };
    in
    map (assertion: assertion.message) (
      lib.filter (
        assertion: !assertion.assertion && lib.hasPrefix "update.mirrors" assertion.message
      ) machine.config.assertions
    );

  cleartext = [
    "http://example.org/alice/nixos.git"
    "git://example.org/alice/nixos.git"
    "mirror.git"
  ];

  accepted = [
    "https://codeberg.org/alice/nixos.git"
    "ssh://git@codeberg.org/alice/nixos.git"
    "git@codeberg.org:alice/nixos.git"
    "file:///srv/mirror.git"
    "/srv/mirror.git"
  ];
in
{
  perSystem =
    { pkgs, system, ... }:
    lib.mkIf (system == "x86_64-linux") {
      checks.feature-auto-upgrade-sources =
        pkgs.runCommand "feature-auto-upgrade-sources"
          {
            nativeBuildInputs = [ pkgs.jq ];
            REFUSED = builtins.toJSON (refusals cleartext);
            ACCEPTED = builtins.toJSON (refusals accepted);
            CLEARTEXT = builtins.toJSON cleartext;
          }
          ''
            jq -e 'length == 1' <<<"$REFUSED" >/dev/null || { echo "FAIL: cleartext mirrors raised no single assertion: $REFUSED" >&2; exit 1; }
            for url in $(jq -r '.[]' <<<"$CLEARTEXT"); do
              jq -e --arg url "$url" '.[0] | contains("\"" + $url + "\"")' <<<"$REFUSED" >/dev/null || { echo "FAIL: the message does not name $url: $REFUSED" >&2; exit 1; }
            done
            jq -e 'length == 0' <<<"$ACCEPTED" >/dev/null || { echo "FAIL: an encrypted mirror was refused: $ACCEPTED" >&2; exit 1; }
            touch $out
          '';
    };
}
