{
  apps.gamemode.nixos =
    { lib, config, ... }:
    {
      programs.gamemode.enable = true;

      users.groups.gamemode.members = lib.attrNames (
        lib.filterAttrs (_: user: user.isNormalUser) config.users.users
      );
    };
}
