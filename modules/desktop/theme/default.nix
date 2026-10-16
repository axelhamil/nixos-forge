{ config, ... }:

let
  inherit (config.helpers) mkDefaults;
in
{
  flake.modules.nixos.theme =
    { pkgs, ... }:
    {
      fonts = {
        packages = with pkgs; [
          noto-fonts
          noto-fonts-color-emoji
          nerd-fonts.jetbrains-mono
        ];

        fontconfig.defaultFonts = {
          sansSerif = [ "Noto Sans" ];
          serif = [ "Noto Serif" ];
          monospace = [ "JetBrainsMono Nerd Font" ];
          emoji = [ "Noto Color Emoji" ];
        };
      };

      programs.dconf.enable = true;
    };

  flake.modules.homeManager.theme =
    { lib, pkgs, ... }:
    {
      options.theme = {
        palette = lib.mkOption {
          type = lib.types.attrsOf lib.types.str;
          description = "Colors every desktop module reads, as hex without `#`.";
        };

        font = {
          sans = lib.mkOption {
            type = lib.types.str;
            default = "Noto Sans";
            description = "Interface font.";
          };

          monospace = lib.mkOption {
            type = lib.types.str;
            default = "JetBrainsMono Nerd Font";
            description = "Terminal and bar font, with Nerd Font icons.";
          };

          size = lib.mkOption {
            type = lib.types.int;
            default = 11;
            description = "Base font size in points.";
          };
        };
      };

      config = {
        theme.palette = mkDefaults {
          background = "1e1e1e";
          surface = "303030";
          overlay = "454545";
          text = "ffffff";
          muted = "9a9996";
          accent = "3584e4";
          urgent = "e01b24";
          warning = "f6d32d";
          success = "33d17a";
        };

        gtk = {
          enable = true;
          colorScheme = lib.mkDefault "dark";
          theme = mkDefaults {
            name = "adw-gtk3-dark";
            package = pkgs.adw-gtk3;
          };
          iconTheme = mkDefaults {
            name = "Papirus-Dark";
            package = pkgs.papirus-icon-theme;
          };
        };

        qt = mkDefaults {
          enable = true;
          platformTheme.name = "gtk3";
        };

        home.pointerCursor = mkDefaults {
          enable = true;
          name = "Adwaita";
          package = pkgs.adwaita-icon-theme;
          size = 24;
          gtk.enable = true;
        };
      };
    };
}
