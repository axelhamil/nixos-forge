{ config, ... }:

let
  inherit (config.helpers) mkDefaults relativeToHome;
  findFiles = "fd --type f --hidden --exclude .git --strip-cwd-prefix";
in
{
  apps.cli.homeManager =
    { config, ... }:
    {
      imports = [
        (mkDefaults {
          programs = {
            fzf = {
              enable = true;
              defaultCommand = findFiles;
              fileWidget.command = findFiles;
              changeDirWidget.command = "fd --type d --hidden --exclude .git --strip-cwd-prefix";
            };
            zoxide.enable = true;
            eza = {
              enable = true;
              git = true;
              enableZshIntegration = false;
            };
            bat.enable = true;
            ripgrep.enable = true;
            fd.enable = true;
            btop.enable = true;
            jq.enable = true;
          };

          home.shellAliases = {
            ll = "eza -l";
            la = "eza -la";
            lt = "eza --tree";
          };
        })
      ];

      home.persistence."/persist".directories = [
        (relativeToHome config "${config.xdg.dataHome}/zoxide")
      ];
    };
}
