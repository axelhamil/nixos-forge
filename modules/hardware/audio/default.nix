{
  flake.modules.nixos.audio =
    { pkgs, ... }:
    {
      services.pipewire = {
        enable = true;
        alsa.enable = true;
        alsa.support32Bit = pkgs.stdenv.hostPlatform.isx86_64;
        pulse.enable = true;
      };

      security.rtkit.enable = true;
    };
}
