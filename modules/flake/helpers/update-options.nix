{ lib, ... }:

let
  inherit (lib) mkOption types;
in
{
  helpers.updateOptions = {
    flake = mkOption {
      type = types.nullOr types.str;
      default = null;
      example = "/home/<user>/nixos";
      description = "Where the machine updates from: a git checkout on the machine, which `forge update` pulls then switches to, or a flake reference such as `github:<owner>/<repo>`. The `auto-upgrade` feature pulls the `deploy/<host>` branch of that fork (of the checkout's `origin` remote). A checkout is kept across boots on a wiped `/`, and in a wiped home.";
    };

    mirrors = mkOption {
      type = types.listOf types.str;
      default = [ ];
      example = [ "https://codeberg.org/<owner>/<repo>.git" ];
      description = "Extra git URLs of the fork, tried in order when the first one is unreachable: `https://`, `ssh://`, an scp-like `user@host:path`, `file://` or an absolute path. `http://` and `git://` are refused, since they are not encrypted. `auto-upgrade` runs as root, so an ssh mirror needs a root SSH key that survives a reboot and the mirror's host in `programs.ssh.knownHosts`, and warns otherwise.";
    };

    signers = mkOption {
      type = types.listOf (
        types.strMatching "(ssh|ecdsa|sk)-[a-z0-9@.-]+ [A-Za-z0-9+/]+=*( [^\n]*)?"
        // {
          description = "SSH public key (an ssh-*, ecdsa-* or sk-* type, the base64 key, an optional comment)";
        }
      );
      default = [ ];
      example = [ "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIExampleKeyOfTheDeploySecret deploy" ];
      description = "SSH public keys that sign the deploy commits CI promotes (the secret `DEPLOY_SIGNING_KEY`, which `nix run .#init` generates). When set, `auto-upgrade` applies a `deploy/<host>` commit only when it carries a valid signature from one of them, and skips a source that serves anything else. Empty, it trusts whatever its sources serve, and `forge doctor` warns.";
    };
  };
}
