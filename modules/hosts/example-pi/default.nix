{ config, ... }:

{
  configurations.nixos.example-pi = {
    system = "aarch64-linux";

    module = {
      imports = with config.flake.modules.nixos; [
        base
        raspberry-pi-5
        pi
        ssh
        alice
      ];

      networking.hostName = "example-pi";

      time.timeZone = "UTC";
      i18n.defaultLocale = "en_US.UTF-8";
      console.keyMap = "us";

      system.stateVersion = "26.05";
    };
  };
}
