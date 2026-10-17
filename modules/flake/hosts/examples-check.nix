{
  lib,
  config,
  options,
  ...
}:

let
  hostsDir = ../../hosts;

  exampleHosts = lib.filter (name: lib.hasPrefix "example" name) (
    lib.attrNames (lib.filterAttrs (_: type: type == "directory") (builtins.readDir hostsDir))
  );

  profiles = lib.attrNames config.profiles;

  listedNames = lib.concatMap lib.attrNames [
    config.features
    config.hardware
    config.apps
  ];

  hostType = options.configurations.nixos.type.nestedTypes.elemType;
  hostFields = lib.filter (name: name != "_module") (lib.attrNames (hostType.getSubOptions [ ]));
  userFields = lib.filter (name: name != "_module") (
    lib.attrNames ((hostType.getSubOptions [ ]).users.type.nestedTypes.elemType.getSubOptions [ ])
  );

  missing =
    host:
    let
      lines = lib.splitString "\n" (builtins.readFile (hostsDir + "/${host}/default.nix"));
      hasLine = regex: lib.any (line: builtins.match regex line != null) lines;
      field = name: hasLine " *(# )?${name}(\\.[^ ]+)? = .*";
    in
    map (name: "the name \"${name}\"") (
      lib.filter (name: !hasLine " *(# )?\"${name}\" #.*") listedNames
    )
    ++ map (name: "the profile \"${name}\"") (lib.filter (name: !hasLine ".*\"${name}\".*") profiles)
    ++ map (name: "the field `${name}`") (lib.filter (name: !field name) hostFields)
    ++ map (name: "the user field `${name}`") (lib.filter (name: !field name) userFields);

  report = lib.concatMap (
    host: map (item: "modules/hosts/${host}/default.nix lacks a line for ${item}") (missing host)
  ) exampleHosts;
in
{
  perSystem =
    { pkgs, system, ... }:
    lib.mkIf (system == "x86_64-linux" && exampleHosts != [ ]) {
      checks.example-hosts =
        assert lib.assertMsg (report == [ ]) (
          "every example host names every field, profile, feature, hardware module and app, enabled or commented out:\n"
          + lib.concatStringsSep "\n" report
        );
        pkgs.writeText "example-hosts" (lib.concatStringsSep "\n" exampleHosts);
    };
}
