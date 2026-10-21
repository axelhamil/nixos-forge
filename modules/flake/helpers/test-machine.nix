{ lib, config, ... }:

let
  inherit (config.helpers) testMachineScript;

  diskoLauncher = ''machine = create_machine(start_command=" ".join(start_command), **kwargs)'';
  launcherWithoutNetworkCard = "machine = create_machine(start_command=machine_command(*start_command), **kwargs)";

  withoutNetworkCard =
    script:
    lib.throwIfNot (lib.hasInfix diskoLauncher script)
      "helpers.diskTest: disko no longer starts its booted machine with `${diskoLauncher}`, so it may get QEMU's default network card; update the launcher in modules/flake/helpers/test-machine.nix"
      (builtins.replaceStrings [ diskoLauncher ] [ launcherWithoutNetworkCard ] script);

  diskTestModule =
    seconds: test:
    { config, ... }:
    {
      globalTimeout = seconds;
      testScript = lib.mkForce (
        { nodes, ... }:
        testMachineScript config.hostPkgs + withoutNetworkCard (test.config.testScript { inherit nodes; })
      );
    };
in
{
  helpers.qemuBinary =
    pkgs:
    (import "${pkgs.path}/nixos/lib/qemu-common.nix" { inherit (pkgs) lib stdenv; }).qemuBinary
      pkgs.qemu_test;

  helpers.testMachineScript = pkgs: ''
    qemu = "${config.helpers.qemuBinary pkgs}"
    firmware = "${pkgs.OVMF.firmware}"
    firmware_vars = "${pkgs.OVMF.variables}"
    ${builtins.readFile ./test-machine.py}
  '';

  helpers.diskTest = seconds: test: test.extend { modules = [ (diskTestModule seconds test) ]; };
}
