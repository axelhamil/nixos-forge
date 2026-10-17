{
  lib,
  config,
  options,
  ...
}:

let
  hostsDir = ../../hosts;
  usersDir = ../../users;

  exampleHosts = lib.filter (name: lib.hasPrefix "example" name) (
    lib.attrNames (lib.filterAttrs (_: type: type == "directory") (builtins.readDir hostsDir))
  );

  profiles = lib.attrNames config.profiles;

  listedNames = lib.concatMap lib.attrNames [
    config.features
    config.hardware
    config.apps
  ];

  fieldsOf = type: lib.filter (name: name != "_module") (lib.attrNames (type.getSubOptions [ ]));

  hostType = options.configurations.nixos.type.nestedTypes.elemType;
  hostFields = fieldsOf hostType;
  userFields = fieldsOf (hostType.getSubOptions [ ]).users.type.nestedTypes.elemType;
  personFields = fieldsOf options.users.type.nestedTypes.elemType;

  examplePeople = lib.filter (name: builtins.pathExists (usersDir + "/${name}/default.nix")) (
    lib.attrNames (config.configurations.nixos.example.users or { })
  );

  linesOf = file: lib.splitString "\n" (builtins.readFile file);

  hasLine = lines: regex: lib.any (line: builtins.match regex line != null) lines;

  hasField = lines: name: hasLine lines " *(# )?${name}(\\.[^ ]+)? = .*";

  missingInPerson =
    person:
    let
      lines = linesOf (usersDir + "/${person}/default.nix");
    in
    map (name: "the field `${name}`") (lib.filter (name: !hasField lines name) personFields);

  missing =
    host:
    let
      lines = linesOf (hostsDir + "/${host}/default.nix");
      lacks = regex: !hasLine lines regex;
      field = hasField lines;
    in
    map (name: "the name \"${name}\"") (lib.filter (name: lacks " *(# )?\"${name}\" #.*") listedNames)
    ++ map (name: "the profile \"${name}\"") (lib.filter (name: lacks ".*\"${name}\".*") profiles)
    ++ map (name: "the field `${name}`") (lib.filter (name: !field name) hostFields)
    ++ map (name: "the user field `${name}`") (lib.filter (name: !field name) userFields);

  report =
    lib.concatMap (
      host: map (item: "modules/hosts/${host}/default.nix lacks a line for ${item}") (missing host)
    ) exampleHosts
    ++ lib.concatMap (
      person:
      map (item: "modules/users/${person}/default.nix lacks a line for ${item}") (missingInPerson person)
    ) examplePeople;
in
{
  perSystem =
    { pkgs, system, ... }:
    lib.mkIf (system == "x86_64-linux" && exampleHosts != [ ]) {
      checks.example-hosts =
        assert lib.assertMsg (report == [ ]) (
          "every example host names every field, profile, feature, hardware module and app, and the example person every field, enabled or commented out:\n"
          + lib.concatStringsSep "\n" report
        );
        pkgs.writeText "example-hosts" (lib.concatLines (exampleHosts ++ examplePeople));
    };
}
