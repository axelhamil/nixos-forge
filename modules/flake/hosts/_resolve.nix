{
  lib,
  inputs,
  config,
}:

let
  inherit (config.helpers) appPart;
  inherit (config) apps features hardware;

  fail =
    host: field: message:
    throw "configurations.nixos.${host}.${field}: ${message}";

  known = namespace: lib.concatStringsSep ", " (lib.attrNames namespace);

  pick =
    host: field: namespace: name:
    namespace.${name} or (fail host field "unknown name \"${name}\". Known: ${known namespace}.");

  hardwareModule =
    host: field: name:
    hardware.${name} or inputs.nixos-hardware.nixosModules.${name} or (fail host field
      "unknown name \"${name}\": neither a module of modules/features/hardware (${known hardware}) nor a nixos-hardware profile (https://github.com/NixOS/nixos-hardware)."
    );

  configuredApps = lib.filter (name: apps ? ${name});
  packagedApps = lib.filter (name: !(apps ? ${name}));

  appModules = part: names: map (name: appPart name part) (configuredApps names);

  package =
    where: pkgs: name:
    lib.attrByPath (lib.splitString "." name)
      (throw "${where}: unknown name \"${name}\": neither an app of modules/apps (${known apps}) nor a package of the host's pkgs (nixpkgs with its overlays); any attribute path of pkgs works, such as \"kdePackages.kate\".")
      pkgs;

  unfreeNames =
    system: overlays: name:
    let
      path = lib.splitString "." name;
      plain = inputs.nixpkgs.legacyPackages.${system};
      resolved = if overlays == [ ] then plain else plain.appendOverlays overlays;
    in
    lib.optional (lib.hasAttrByPath path resolved) (lib.getName (lib.getAttrFromPath path resolved));

  selection =
    host: system: prefix: chosen:
    let
      field = name: "${prefix}${name}";
      where = "configurations.nixos.${host}.${field "apps"}";
      packages = packagedApps chosen.apps;
    in
    { pkgs, config, ... }:
    {
      imports =
        map (pick host (field "features") features) chosen.features
        ++ map (hardwareModule host (field "hardware")) chosen.hardware
        ++ appModules "nixos" chosen.apps;

      environment.systemPackages = map (package where pkgs) packages;
      home-manager.sharedModules = appModules "homeManager" chosen.apps;
      nixpkgs.config.allowUnfreePackages = lib.concatMap (unfreeNames system config.nixpkgs.overlays) packages;
    };
in
{
  inherit
    fail
    known
    pick
    appModules
    packagedApps
    package
    unfreeNames
    selection
    ;
}
