{ config, ... }:

{
  perSystem = config.helpers.appCheck "upgrade-notify" {
    machine =
      { pkgs, ... }:
      {
        environment.systemPackages = [
          pkgs.jq
          pkgs.libnotify
          (pkgs.writers.writePython3Bin "notification-stub" {
            libraries = [ pkgs.python3Packages.dbus-next ];
            flakeIgnore = [
              "E501"
              "F722"
              "F821"
            ];
          } (builtins.readFile ./notification-stub.py))
        ];
      };
    testScripts = [ ./test.py ];
  };
}
