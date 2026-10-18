{
  pkgs,
  gh ? pkgs.gh,
}:

pkgs.writeShellApplication {
  name = "narvi";
  runtimeInputs = [
    pkgs.curl
    pkgs.jq
    gh
    pkgs.coreutils
    pkgs.gnugrep
  ];
  runtimeEnv.MESSAGES = ./messages.json;
  excludeShellChecks = [ "SC2016" ];
  text = builtins.readFile ./narvi.sh;
}
