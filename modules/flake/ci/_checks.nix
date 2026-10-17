{ lib }:

let
  hostTests = [
    {
      suffix = "toplevel";
      proves = "the system builds";
    }
    {
      suffix = "boots";
      proves = "it starts and every user can log in";
    }
    {
      suffix = "disk";
      proves = "it installs on an encrypted disk, wipes / at boot and restores a snapshot";
    }
    {
      suffix = "wipe-home";
      proves = "/home survives a reboot";
    }
    {
      suffix = "installer";
      proves = "the USB installer installs it";
    }
    {
      suffix = "sd-image";
      proves = "the SD card image builds";
    }
  ];

  moduleLayers = [
    "system"
    "profile"
    "app"
    "dev"
    "home"
    "desktop"
  ];

  otherChecks = {
    treefmt = "the code is formatted";
    facter = "a hardware report from the installer configures the machine";
    options-doc = "docs/options.md matches the host options";
    init = "`nix run .#init` writes a valid first host";
    ci-report = "the CI report reads every result correctly";
    example-hosts = "every example host lists every option the starter ships";
  };

  provesOnHost =
    hostChecks: check:
    let
      host = lib.findFirst (name: lib.elem check hostChecks.${name}) null (lib.attrNames hostChecks);
      test = lib.findFirst (test: "${host}-${test.suffix}" == check) null hostTests;
    in
    if host == null then null else test.proves;

  provesAsModule =
    check:
    let
      parts = builtins.match "([a-z]+)-(.+)" check;
    in
    if parts != null && lib.elem (lib.head parts) moduleLayers then
      "${lib.last parts} works in a VM"
    else
      null;
in
{
  hostChecksOf =
    checks: name:
    lib.filter (check: checks ? ${check}) (map (test: "${name}-${test.suffix}") hostTests);

  describe =
    hostChecks: check:
    lib.findFirst (proves: proves != null) "its own test passes" [
      (otherChecks.${check} or null)
      (provesOnHost hostChecks check)
      (provesAsModule check)
    ];
}
