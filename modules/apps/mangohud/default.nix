{ config, ... }:

let
  inherit (config.helpers) mkDefaults relativeToHome;
in
{
  apps.mangohud.homeManager =
    { config, ... }:
    let
      logs = "${config.xdg.stateHome}/mangohud";
    in
    {
      programs.mangohud = mkDefaults {
        enable = true;
        settings = {
          fps = true;
          frametime = true;
          frame_timing = true;
          fps_metrics = "avg,0.01";
          cpu_stats = true;
          gpu_stats = true;
          ram = true;
          vram = true;
          toggle_logging = "Shift_L+F2";
          output_folder = logs;
        };
      };

      systemd.user.tmpfiles.rules = [ "d ${logs} - - - -" ];
      home.persistence."/persist".directories = [ (relativeToHome config logs) ];
    };
}
