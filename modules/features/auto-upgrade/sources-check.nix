{
  lib,
  inputs,
  config,
  ...
}:

let
  inherit (config) base features;

  outcome =
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
      aboutMirrors = lib.hasPrefix "update.mirrors";
    in
    {
      refused = map (assertion: assertion.message) (
        lib.filter (
          assertion: !assertion.assertion && aboutMirrors assertion.message
        ) machine.config.assertions
      );
      warned = lib.filter aboutMirrors machine.config.warnings;
    };

  cleartext = [
    "http://example.org/alice/nixos.git"
    "git://example.org/alice/nixos.git"
    "mirror.git"
  ];

  overSsh = [
    "ssh://git@codeberg.org/alice/nixos.git"
    "git@codeberg.org:alice/nixos.git"
    "nas:/srv/mirror.git"
  ];

  accepted = [
    "https://codeberg.org/alice/nixos.git"
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
            CLEARTEXT_OUTCOME = builtins.toJSON (outcome cleartext);
            OVER_SSH_OUTCOME = builtins.toJSON (outcome overSsh);
            ACCEPTED_OUTCOME = builtins.toJSON (outcome accepted);
            CLEARTEXT = builtins.toJSON cleartext;
            OVER_SSH = builtins.toJSON overSsh;
          }
          ''
            jq -e '(.refused | length) == 1 and .warned == []' <<<"$CLEARTEXT_OUTCOME" >/dev/null || { echo "FAIL: cleartext mirrors raised no single assertion: $CLEARTEXT_OUTCOME" >&2; exit 1; }
            for url in $(jq -r '.[]' <<<"$CLEARTEXT"); do
              jq -e --arg url "$url" '.refused[0] | contains("\"" + $url + "\"")' <<<"$CLEARTEXT_OUTCOME" >/dev/null || { echo "FAIL: the message does not name $url: $CLEARTEXT_OUTCOME" >&2; exit 1; }
            done
            jq -e '.refused == [] and (.warned | length) == 1' <<<"$OVER_SSH_OUTCOME" >/dev/null || { echo "FAIL: ssh mirrors were refused or raised no single warning: $OVER_SSH_OUTCOME" >&2; exit 1; }
            for url in $(jq -r '.[]' <<<"$OVER_SSH"); do
              jq -e --arg url "$url" '.warned[0] | contains("\"" + $url + "\"") and contains("programs.ssh.knownHosts")' <<<"$OVER_SSH_OUTCOME" >/dev/null || { echo "FAIL: the warning does not name $url or the known host it needs: $OVER_SSH_OUTCOME" >&2; exit 1; }
            done
            jq -e '.refused == [] and .warned == []' <<<"$ACCEPTED_OUTCOME" >/dev/null || { echo "FAIL: an https or local mirror was refused or warned about: $ACCEPTED_OUTCOME" >&2; exit 1; }
            touch $out
          '';
    };
}
