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
      proves = "it installs on an encrypted disk, wipes / at boot and takes a snapshot";
    }
    {
      suffix = "wipe-home";
      proves = "with wipe-home added, it installs and starts each boot with an empty /home";
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
    options-doc = "docs/reference/options.md matches the host options";
    docs = "every link and anchor of the docs resolves, every page is in the docs index, and no new page names an example of the starter or leaves a module without a section";
    init = "`nix run .#init` writes a first host and person whose system evaluates";
    ci-plan = "CI builds every check the binary cache lacks, skips host kernels in a pull request only, and never skips a check that compares with the base";
    ci-report = "the CI report reads every result correctly";
    ci-badges = "the live badges count the checks, VM tests, hosts, boot time and reproducibility of a run, and a publish keeps the badges it does not write";
    reproducibility = "`nix run .#reproducibility` reports an identical rebuild, and a differing one with its diffoscope summary";
    ci-promote = "a machine's deploy branch moves only once every check of that machine passed, signed when CI holds the key, never on a tip the key did not sign";
    feature-auto-upgrade-signed = "a machine with signers applies only a deploy commit they signed for it, never an older one";
    feature-auto-upgrade-sources = "a mirror over http:// or git:// stops the build, and one over ssh warns that it needs a root key and known host";
    ci-mirror = "main and the deploy branches reach the mirror, and nothing else";
    try-install = "`nix run .#try-install` refuses a device, an unknown host or a host without an installer before touching anything";
    upstream = "`nix run .#upstream` lists the starter's breaking changes and merges its release into a branch whose pull request lists the upstream commits, or stops cleanly on a conflict";
    example-hosts = "every example host and the example person list every option the starter ships";
    wrong-namespace = "a name of another registry field, or an unknown person, stops the build with the field's own names";
    apps-once = "an app named by the host and the person is imported once, and an unfree package of an overlay works as an app";
    fork-overrides = "a fork overrides the shipped policy values with plain definitions, a nixos-hardware profile keeps its NVIDIA choice, and firmware and plymouth evaluate on a Raspberry Pi";
    hyprland-alone = "hyprland alone evaluates, warns that no terminal is registered, and binds no key to an app or to audio the host lacks";
    feature-dual-boot = "Windows on its own disk starts from the systemd-boot menu under Secure Boot, with or without BitLocker or its firmware entry, its disk unchanged and readable from NixOS read-only";
    feature-dual-boot-same-disk = "NixOS installs beside Windows on one disk over an old LUKS header, refuses an ESP too small, keeps the Windows boot manager when another Windows disk comes and goes, and starts Windows from its menu under Secure Boot with the Windows partitions unchanged";
    feature-dual-boot-systemd-boot = "without secure-boot, Windows on its own disk starts from the plain systemd-boot menu, with or without BitLocker or its firmware entry";
    feature-dual-boot-hook-failure = "with lanzaboote, a boot loader install that fails makes the switch fail instead of passing";
    feature-migration = "a machine running `example` from the base revision keeps its state after switching to this version";
    perf-regression = "the base revision and this one boot in turn in one VM, measured for boot time, idle RAM, closure size, running services and CPU, memory and disk benchmarks";
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
