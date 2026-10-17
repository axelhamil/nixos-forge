{
  base = {
    nix.settings = {
      experimental-features = [
        "nix-command"
        "flakes"
      ];
      trusted-users = [ "@wheel" ];
      use-xdg-base-directories = true;
    };

    nix.channel.enable = false;

    nix.optimise.automatic = true;

    programs.nh = {
      enable = true;
      clean = {
        enable = true;
        extraArgs = "--keep-since 14d --keep 5 --keep-one";
      };
    };
  };
}
