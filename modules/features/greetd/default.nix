{
  features.greetd =
    {
      lib,
      pkgs,
      config,
      ...
    }:
    let
      inherit (config.services.displayManager.sessionData) desktops;
      sessionKinds = [
        "wayland-sessions"
        "xsessions"
      ];

      tuigreet = lib.escapeShellArgs (
        [
          (lib.getExe pkgs.tuigreet)
          "--time"
          "--remember"
          "--remember-user-session"
          "--asterisks"
          "--sessions"
          "/etc/greetd/wayland-sessions"
          "--xsessions"
          "/etc/greetd/xsessions"
        ]
        ++ lib.optionals (config.greetd.command != null) [
          "--cmd"
          config.greetd.command
        ]
      );
    in
    {
      options.greetd.command = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        description = "Session the greeter starts when the user picked none and none is remembered.";
      };

      config = {
        services.greetd = {
          enable = true;
          useTextGreeter = true;
          settings.default_session.command = tuigreet;
        };

        environment.etc = lib.genAttrs' sessionKinds (
          kind: lib.nameValuePair "greetd/${kind}" { source = "${desktops}/share/${kind}"; }
        );

        environment.persistence."/persist".directories = [
          {
            directory = "/var/cache/tuigreet";
            user = "greeter";
            group = "greeter";
          }
        ];
      };
    };
}
