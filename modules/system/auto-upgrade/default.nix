{
  flake.modules.nixos.auto-upgrade =
    { lib, config, ... }:
    let
      inherit (config.update) flake;
    in
    {
      assertions = [
        {
          assertion = flake != null && !lib.hasPrefix "/" flake;
          message = "auto-upgrade builds from a flake reference such as github:alice/nixos, set in the host's `flake` field; a checkout on the machine is updated with `update`.";
        }
      ];

      system.autoUpgrade = {
        enable = true;
        inherit flake;
        dates = "Tue 04:00";
        randomizedDelaySec = "45min";
      };
    };
}
