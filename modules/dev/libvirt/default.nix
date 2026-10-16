{
  flake.modules.nixos.libvirt =
    { pkgs, ... }:
    {
      virtualisation.libvirtd = {
        enable = true;
        qemu = {
          runAsRoot = false;
          swtpm.enable = true;
          vhostUserPackages = [ pkgs.virtiofsd ];
        };
      };

      virtualisation.spiceUSBRedirection.enable = true;

      programs.virt-manager.enable = true;

      networking.firewall.trustedInterfaces = [ "virbr0" ];

      systemd.tmpfiles.settings.libvirt-images."/var/lib/libvirt/images" = {
        v.mode = "0711";
        h.argument = "+C";
      };

      environment.persistence."/persist".directories = [
        "/var/lib/libvirt"
        "/var/lib/swtpm-localca"
      ];
    };
}
