{ config, ... }:

let
  inherit (config.helpers) flakePackage relativeToHome;
in
{
  apps.stow.homeManager =
    {
      config,
      lib,
      pkgs,
      ...
    }:
    let
      cfg = config.stow;

      forge-apps = flakePackage pkgs "forge-apps";

      script = pkgs.writeShellApplication {
        name = "stow-dotfiles";
        runtimeInputs = [ pkgs.stow ];
        text = ''exec ${lib.getExe' forge-apps "stow-dotfiles"} "$@"'';
      };

      stowArgs =
        target:
        "${cfg.directory} ${target} ${config.xdg.stateHome}/stow/packages "
        + lib.escapeShellArgs (lib.optional cfg.dotfiles "--dotfiles" ++ cfg.packages);

      stowDotfiles = mode: target: "${lib.getExe script} ${mode} ${stowArgs target}";
    in
    {
      options.stow = {
        directory = lib.mkOption {
          type = lib.types.str;
          default = "${config.home.homeDirectory}/dotfiles";
          description = "Path to your dotfiles clone: a mirror of your home, or one directory per stow package.";
        };

        packages = lib.mkOption {
          type = lib.types.listOf lib.types.str;
          default = [ ];
          description = "Stow packages to link; empty links the mirror of your home, or else every top-level directory.";
        };

        dotfiles = lib.mkOption {
          type = lib.types.bool;
          default = false;
          description = "Pass --dotfiles, so a dot- prefix in your repository becomes a dot.";
        };
      };

      config = {
        home.packages = [ pkgs.stow ];

        home.persistence."/persist".directories = map (relativeToHome config) (
          [ "${config.xdg.stateHome}/stow" ]
          ++ lib.optional (lib.hasPrefix "${config.home.homeDirectory}/" cfg.directory) cfg.directory
        );

        home.activation = {
          stowPrune = lib.hm.dag.entryBefore [ "stowCheck" ] ''
            run ${stowDotfiles "prune" "$HOME"}
          '';

          stowCheck = lib.hm.dag.entryBefore [ "checkLinkTargets" ] ''
            ${stowDotfiles "check" "$newGenPath/home-files"}
          '';

          stow = lib.hm.dag.entryAfter [ "linkGeneration" ] ''
            run ${stowDotfiles "apply" "$HOME"}
          '';
        };
      };
    };
}
