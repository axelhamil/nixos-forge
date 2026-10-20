# Variants

One machine, several setups: a quiet one for work, one for games. A variant is the whole host plus a few names of its own, built with the host, listed in the boot menu, and switched to without a reboot.

## Declare one

```nix
configurations.nixos.<host> = {
  # ... the host's fields
  specialisations.gaming = {
    apps = [ "steam" "gamemode" "mangohud" ];
  };
};
```

A variant takes `features`, `apps`, `hardware` and `module`, by the same names as the host and added to the host's. An unknown name stops the build and names the field, such as `configurations.nixos.<host>.specialisations.gaming.features`. Anything else goes in its `module`, plain NixOS config; to change a value the host sets, the host sets it with `lib.mkDefault`.

A variant is a NixOS [specialisation](https://wiki.nixos.org/wiki/Specialisation) that inherits the host: one build gives the host and each of its variants.

## Switch

| To | Run |
|---|---|
| go to a variant now | `update -s gaming` |
| go back to the host now | `update -S` |
| start a variant at boot | pick its entry, named after it, in the boot menu |

The running variant's name is in `/etc/specialisation` (no file on the host itself). `update` keeps it: on `gaming`, a plain `update` pulls, builds and stays on `gaming`. `auto-upgrade` keeps it too when its `operation` is `switch`; with the default `boot`, the next boot starts the host unless you pick the variant again. A switch lasts until the next reboot, which starts the default entry, the host. When the running variant no longer exists in your config, `update` stops and says so: run `update -S` to switch to the host; `auto-upgrade` switches to the host on its own.

A switch does not reload the kernel. A variant that changes the kernel or its modules (another NVIDIA driver, another kernel) takes effect only when it is started from the boot menu.

With `secure-boot`, lanzaboote signs an entry for each variant, like systemd-boot without it.

## Games

| Name | What it brings |
|---|---|
| `steam` (app) | Steam with [Proton-GE](https://github.com/GloriousEggroll/proton-ge-custom) and protontricks, 32-bit graphics, the `ntsync` kernel module that recent Wine and Proton use for Windows synchronization when `/dev/ntsync` exists, its library in `~/.local/share/Steam` kept with `wipe-home` |
| `gamemode` (app) | [Feral GameMode](https://github.com/FeralInteractive/gamemode): put `gamemoderun %command%` in a game's launch options; every user is in the `gamemode` group, which lets it set the CPU governor |
| `mangohud` (app) | an overlay with FPS, frame times, the average and 1% low FPS, CPU and GPU load: put `mangohud %command%` in a game's launch options; Shift+F2 starts and stops a log in `~/.local/state/mangohud`, kept with `wipe-home` |

Steam's unfree packages are allowed by the app. The GPU driver stays the host's `hardware` (`nvidia`, `amd-gpu`, `intel-gpu`), so the variant changes no kernel and switches without a reboot. The `desktop` profile already runs the `scx_lavd` scheduler, meant for games ([performance.md](../reference/performance.md)).

No gaming kernel ships here: we found no public benchmark showing a gain in games over the default kernel, and the variant above already runs the `scx_lavd` scheduler of the `desktop` profile. `pkgs.linuxPackages_zen` comes from nixpkgs and its binary cache; the CachyOS kernel comes from [chaotic-nyx](https://github.com/chaotic-cx/nyx) or [nix-cachyos-kernel](https://github.com/xddxdd/nix-cachyos-kernel), each with its own cache to trust. A variant's `module` can set `boot.kernelPackages = pkgs.linuxPackages_zen;`, which applies from the boot menu only. Before keeping it, measure: play the same scene in both setups with Shift+F2, then compare the average and the 1% low FPS of the two logs; a gain smaller than the spread between two runs of the same setup is noise.
