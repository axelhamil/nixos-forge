{ inputs, config, ... }:

{
  helpers.layerMachine =
    system: modules:
    inputs.nixpkgs.lib.nixosSystem {
      modules = [
        config.base
        {
          nixpkgs.hostPlatform = system;
          system.stateVersion = config.helpers.testStateVersion;
        }
      ]
      ++ modules;
    };
}
