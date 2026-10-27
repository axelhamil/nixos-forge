# A fork

The commented [`example`](modules/hosts/example/default.nix) host lists every profile, feature, hardware module and app nixos-forge ships, one line each to uncomment.

1. Run `nix run .#init`, which:
   - writes `modules/hosts/<host>/default.nix` from the [example host](modules/hosts/example/default.nix) and `modules/users/<login>/default.nix` from the [example person](modules/users/alice/default.nix);
   - removes the examples.
