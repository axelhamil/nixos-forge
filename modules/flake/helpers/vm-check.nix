{ lib, config, ... }:

let
  inherit (config.helpers) appPart testStateVersion;
  inherit (config)
    base
    profiles
    features
    ;

  appNode =
    app: machine:
    { lib, ... }:
    {
      imports = [
        base
        (appPart app "nixos")
        machine
      ];
      users.users.tester.isNormalUser = true;
      home-manager.users.tester = {
        imports = [ (appPart app "homeManager") ];
        home.stateVersion = testStateVersion;
      };
      virtualisation.memorySize = lib.mkDefault 2048;
    };

  layerNode =
    layer: name: machine:
    {
      base.imports = [
        base
        machine
      ];
      feature.imports = [
        base
        features.${name}
        machine
      ];
      profile.imports = [
        base
        profiles.${name}
        { virtualisation.cores = 2; }
        machine
      ];
      app = appNode name machine;
    }
    .${layer}
      or (throw "helpers.moduleCheck: layer is one of base, feature, profile, app, not ${layer}");
in
{
  helpers.testRevision = {
    system.configurationRevision = lib.mkForce "test";
  };

  helpers.vmCheck =
    name:
    {
      nodes,
      qemu ? null,
      globalTimeout ? 400,
      testScripts,
    }:
    {
      lib,
      pkgs,
      system,
      ...
    }:
    lib.mkIf (system == "x86_64-linux") {
      checks.${name} = pkgs.testers.runNixOSTest {
        inherit name nodes globalTimeout;
        qemu = lib.optionalAttrs (qemu != null) { package = qemu pkgs; };
        defaults = {
          imports = [ config.helpers.testRevision ];
          system.stateVersion = testStateVersion;
        };
        testScript = lib.concatMapStrings builtins.readFile testScripts;
      };
    };

  helpers.moduleCheck =
    layer: name:
    {
      machine ? { },
      ...
    }@check:
    config.helpers.vmCheck "${layer}-${name}" (
      removeAttrs check [ "machine" ] // { nodes.machine = layerNode layer name machine; }
    );

  helpers.baseCheck = config.helpers.moduleCheck "base";

  helpers.featureCheck = config.helpers.moduleCheck "feature";

  helpers.profileCheck = config.helpers.moduleCheck "profile";

  helpers.appCheck = config.helpers.moduleCheck "app";
}
