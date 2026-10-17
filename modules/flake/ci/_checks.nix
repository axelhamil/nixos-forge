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
    "base"
    "feature"
    "profile"
    "hardware"
    "app"
  ];

  otherChecks = {
    treefmt = "the code is formatted";
    facter = "a hardware report from the installer configures the machine";
    options-doc = "docs/options.md matches the host options";
    init = "`nix run .#init` writes a first host and person whose system evaluates";
    ci-report = "the CI report reads every result correctly";
    ci-badges = "the live badges count the checks, VM tests, hosts, boot time and reproducibility of a run, and a publish keeps the badges it does not write";
    reproducibility = "`nix run .#reproducibility` reports an identical rebuild, and a differing one with its diffoscope summary";
    ci-promote = "a machine's deploy branch moves only once every check of that machine passed, signed when CI holds the key, never on a tip the key did not sign";
    feature-auto-upgrade-signed = "a machine with signers applies only a deploy commit they signed for it, never an older one";
    feature-auto-upgrade-sources = "a mirror over http:// or git:// stops the build";
    ci-mirror = "main and the deploy branches reach the mirror, and nothing else";
    try-install = "`nix run .#try-install` refuses a device, an unknown host or a host without an installer before touching anything";
    upstream = "`nix run .#upstream` lists the starter's breaking changes and merges its release into a branch, or stops cleanly on a conflict";
    example-hosts = "every example host and the example person list every option the starter ships";
    wrong-namespace = "a name of another registry field, or an unknown person, stops the build with the field's own names";
    apps-once = "an app reached twice is imported once";
    feature-migration = "a machine running `example` from the base revision keeps its state after switching to this version";
    feature-migration-without-disk = "an `example` host without `disk`, here or on the base, gets no migration check and still evaluates";
    feature-rollback-failed-unit = "a generation whose service fails after boot is not blessed and is rolled back";
    flake-modules = "no name is published by two namespaces";
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
