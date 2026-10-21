{ config, ... }:

{
  perSystem = config.helpers.featureCheck "fido2" {
    qemu = pkgs: pkgs.qemu_test.override { canokeySupport = true; };
    machine =
      { lib, pkgs, ... }:
      {
        virtualisation = {
          emptyDiskImages = [ 512 ];
          useBootLoader = true;
          useEFIBoot = true;
          mountHostNixStore = true;
          tpm.enable = true;
          qemu.options = [
            "-device pci-ohci,id=usb-bus"
            "-device canokey,bus=usb-bus.0,file=/tmp/canokey-file"
          ];
        };
        boot.loader.systemd-boot.enable = true;
        boot.initrd.systemd.enable = true;
        environment.systemPackages = [ pkgs.cryptsetup ];

        users.users = lib.genAttrs [ "alice" "bob" ] (_: {
          isNormalUser = true;
          extraGroups = [ "wheel" ];
          password = "password";
        });
        fido2.u2fKeys = {
          alice = [ ":KeyHandle,UserKey,es256,+presence" ];
          bob = [ ];
        };

        specialisation.encrypted.configuration = {
          fido2.unlockLuks = true;
          boot.initrd.luks.devices = lib.mkVMOverride {
            cryptroot = {
              device = "/dev/vdb";
              crypttabExtraOpts = [
                "tpm2-device=auto"
                "tpm2-measure-pcr=yes"
              ];
            };
          };
          virtualisation.rootDevice = "/dev/mapper/cryptroot";
        };
      };
    globalTimeout = 600;
    testScripts = [ ./test.py ];
  };
}
