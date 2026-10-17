{
  helpers.storeOnDiskTest =
    { lib, pkgs, ... }:
    {
      disko.tests.extraConfig = {
        fileSystems."/nix/store" = lib.mkOverride 40 {
          device = "nix-store";
          fsType = "9p";
          neededForBoot = true;
          options = [
            "trans=virtio"
            "version=9p2000.L"
            "cache=loose"
            "nofail"
          ];
        };

        boot.initrd.systemd.services.initrd-find-nixos-closure = {
          wants = [ "sysroot-nix-store.mount" ];
          after = [ "sysroot-nix-store.mount" ];
          unitConfig.RequiresMountsFor = lib.mkForce "/sysroot/nix";
        };
      };

      disko.tests.extraChecks = lib.mkBefore ''
        cryptsetup = "${lib.getExe' pkgs.cryptsetup "cryptsetup"}"
        ${builtins.readFile ./store-on-disk.py}
      '';
    };
}
