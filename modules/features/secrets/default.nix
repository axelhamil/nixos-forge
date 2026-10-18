{ inputs, ... }:

{
  features.secrets =
    { config, ... }:
    {
      imports = [ inputs.sops-nix.nixosModules.sops ];

      sops.age = {
        keyFile = "${config.impermanence.persistPrefix}/var/lib/sops-nix/key.txt";
        sshKeyPaths = [ ];
      };

      sops.gnupg.sshKeyPaths = [ ];
    };
}
