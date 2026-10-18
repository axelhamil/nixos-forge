# Raspberry Pi

A Raspberry Pi 4 or 5 runs the same starter: same `base`, users and opt-in modules, plus a board module and the `pi` profile. Board support comes from [nixos-raspberrypi](https://github.com/nvmd/nixos-raspberrypi) (vendor kernel, firmware, `config.txt`, bootloader). The host builds an SD card image.

This page takes a Pi from the example host to a flashed card, then keeps it up to date without making it compile anything.

## Use it

[`modules/hosts/example-pi/default.nix`](../modules/hosts/example-pi/default.nix) is the example: `system = "aarch64-linux"`, `profile = "pi"`, the hardware `raspberry-pi-5` (or `raspberry-pi-4`), the feature `ssh`, and a user. Rename the host and the user as for any host, and give your person an SSH key in `modules/users/<you>/default.nix`: a Pi starts headless, and SSH refuses passwords.

```nix
users.<you>.sshKeys = [ "ssh-ed25519 AAAA... you@laptop" ];
```

A fork without a Pi deletes `modules/hosts/example-pi` and the `nixos-raspberrypi` input, which otherwise fetches its own nixpkgs at every evaluation.

## What is in it

- **Board modules** `raspberry-pi-4` and `raspberry-pi-5`: firmware partition on `/boot/firmware`, ext4 root grown to the card at first boot, and the `raspberrypi-eeprom` tools. The kernel and firmware are built from this flake's nixpkgs, so they follow the lock updates. ZFS does not build against the vendor kernel.
- **`pi` profile**: the `server` profile, builds sent to the builders in `nix.buildMachines`, passwordless `sudo` for `wheel` (the only way in is an SSH key), a 15 s hardware watchdog, the journal capped at 100 MB, and no NixOS manual.

The Pi has no `disk`: no LUKS, btrfs or wiped `/` on an SD card. `ssh`, `secrets`, the `audio` hardware module and the apps work as on any host.

## Build and flash the image

The image is an `aarch64-linux` build. On an x86 machine, let it emulate the Pi, in that machine's host config:

```nix
boot.binfmt.emulatedSystems = [ "aarch64-linux" ];
```

Then build it and write it to the card. Check the device with `lsblk` first: everything on it is lost.

```sh
nix build .#nixosConfigurations.example-pi.config.system.build.sdImage
zstdcat result/sd-image/*.img.zst | sudo dd of=/dev/sdX bs=4M status=progress conv=fsync
```

The first build compiles the vendor kernel, which is slow under emulation: a native `aarch64-linux` builder is faster. nixos-raspberrypi's binary cache has no hit for it.

To boot from NVMe on a Pi 5, write the same image to the drive and put NVMe first in the EEPROM boot order (`sudo rpi-eeprom-config --edit`, `BOOT_ORDER=0xf416`).

## Update it

With the `auto-upgrade` feature, the Pi follows `deploy/<host>` on its own ([updates.md](updates.md#a-machine-upgrades-on-its-own)) without building anything: the `pi` profile turns local builds off, so it upgrades only to a version CI built and pushed to your binary cache (`<host>-toplevel`), and waits otherwise.

A Pi has no automatic rollback: its boot loader does not count boot tries, which takes the `disk` feature with systemd-boot, so a new version that does not start stays the default. When it still answers over SSH, `sudo nixos-rebuild switch --rollback` goes back to the previous generation; otherwise write the SD image of a commit that worked. Then push a fix, which CI validates before the Pi takes it.

By hand, build on your machine and push the result, so the Pi never builds anything:

```sh
nh os switch --hostname example-pi --target-host alice@example-pi --elevation-strategy passwordless .
```

For `nh os switch` on the Pi itself, give it a builder. On the builder, `nix.sshServe` opens a Nix-only SSH endpoint (no shell) for the Pi's `root` key:

```nix
nix.sshServe = {
  enable = true;
  write = true;
  trusted = true;
  protocol = "ssh-ng";
  keys = [ "ssh-ed25519 AAAA... root@example-pi" ];
};
```

On the Pi:

```nix
nix.buildMachines = [
  {
    hostName = "builder.lan";
    system = "aarch64-linux";
    protocol = "ssh-ng";
    sshUser = "nix-ssh";
    sshKey = "/etc/ssh/ssh_host_ed25519_key";
    publicHostKey = "<base64 of the builder's /etc/ssh/ssh_host_ed25519_key.pub>";
    maxJobs = 4;
  }
];
```

CI builds the image on an arm runner ([ci.md](ci.md)).
