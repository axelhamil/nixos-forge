# Raspberry Pi

A Raspberry Pi 4 or 5 runs the same starter: same `base`, users and opt-in modules, plus a board module and the `pi` profile. Board support comes from [nixos-raspberrypi](https://github.com/nvmd/nixos-raspberrypi) (vendor kernel, firmware, `config.txt`, bootloader), since nixpkgs has no official Pi 5 support. The host builds an SD card image, and CI builds it on an `aarch64-linux` runner for every PR.

## Use it

[`modules/hosts/example-pi/default.nix`](../modules/hosts/example-pi/default.nix) is the example: `system = "aarch64-linux"` and the imports `base`, `raspberry-pi-5` (or `raspberry-pi-4`), `pi`, `ssh` and your user. Rename the host and the user as for any host, and give your user an SSH key: a Pi starts headless, and SSH refuses passwords.

```nix
users.users.<you>.openssh.authorizedKeys.keys = [ "ssh-ed25519 AAAA... you@laptop" ];
```

A fork without a Pi deletes `modules/hosts/example-pi` and the `nixos-raspberrypi` input: every evaluation of a Pi host fetches nixos-raspberrypi's own nixpkgs, which its library is built on.

## What is in it

- **Board modules** `raspberry-pi-4` and `raspberry-pi-5`: the nixos-raspberrypi board module and the SD card layout (firmware partition on `/boot/firmware`, ext4 root grown to the card at first boot, `noatime`), plus `rpi-eeprom-update` for the bootloader EEPROM. The Pi 5 boots its kernel straight from the firmware with one boot entry per generation, the Pi 4 through U-Boot. The kernel and firmware are built from this flake's nixpkgs with nixos-raspberrypi's overlays, so they follow the weekly lock update like the rest of the system. ZFS is left out: it does not build against the vendor kernel.
- **`pi` profile**: the `server` profile, builds sent to the remote builders you list (`nix.distributedBuilds` turns on with `nix.buildMachines`, and builders fetch from caches themselves), `sudo` without a password for `wheel` (the only way in is an SSH key, and the Pi has no password store), a 15 s hardware watchdog that reboots a hung Pi, the journal capped at 100 MB, and no NixOS manual to build.

The Pi does not import `disk`: no LUKS, btrfs or wiped `/` on an SD card. `ssh`, `secrets`, `audio` and the home modules work as on any host.

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

The first build compiles the vendor kernel, which is slow under emulation. A native `aarch64-linux` machine as builder makes it faster, and a cache on your own machine ([harmonia](https://github.com/nix-community/harmonia), `services.harmonia`) makes later builds a download. nixos-raspberrypi's cache does not help: its kernels come from its own nixpkgs pin.

To boot from NVMe on a Pi 5, write the same image to the drive and put NVMe first in the EEPROM boot order (`sudo rpi-eeprom-config --edit`, `BOOT_ORDER=0xf416`).

## Update it

Build on your machine and push the result, so the Pi never builds anything:

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

## Tests

- `<host>-sd-image` builds the SD card image on an `ubuntu-24.04-arm` runner, in place of `<host>-toplevel` and `<host>-boots`: the image contains the system, and the vendor kernel only boots on the board. Without a binary cache it compiles the kernel on every run, which takes more than an hour (the job allows 240 minutes): set up the optional Cachix cache of [ci.md](ci.md) so later runs download it.
- `profile-pi` checks the `server` settings, the remote builder in `/etc/nix/machines` with `builders-use-substitutes`, passwordless `sudo` for `wheel`, the watchdog and the journal cap.
- The PR package diff covers `aarch64-linux` hosts in a job on the arm runner.
