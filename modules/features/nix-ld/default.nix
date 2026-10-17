{
  features.nix-ld = {
    programs.nix-ld.enable = true;

    programs.appimage = {
      enable = true;
      binfmt = true;
    };
  };
}
