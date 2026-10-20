{
  features.ssh =
    { lib, config, ... }:
    {
      services.openssh = {
        enable = true;

        hostKeys = [
          {
            type = "ed25519";
            path = "${config.impermanence.persistPrefix}/etc/ssh/ssh_host_ed25519_key";
          }
        ];

        settings = {
          PasswordAuthentication = false;
          KbdInteractiveAuthentication = false;
          PermitRootLogin = lib.mkDefault "no";
          KexAlgorithms = lib.mkDefault [
            "mlkem768x25519-sha256"
            "sntrup761x25519-sha512"
            "sntrup761x25519-sha512@openssh.com"
            "curve25519-sha256"
            "curve25519-sha256@libssh.org"
          ];
        };
      };
    };
}
