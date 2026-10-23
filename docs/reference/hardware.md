# Hardware

What each module of `modules/features/hardware` turns on. List the ones your machine needs in the host's `hardware`; a name with no module there resolves to a [nixos-hardware](https://github.com/NixOS/nixos-hardware) profile (`common-pc-ssd`, `framework-13-7040-amd`, ...).

```nix
hardware = [ "firmware" "amd-gpu" "audio" "bluetooth" ];
```

## `firmware`

On a physical machine. It installs the redistributable firmware, loads the Intel and AMD CPU microcode updates (each a `mkDefault`) and runs fwupd, so `fwupdmgr` can update the firmware of the machine and its devices. fwupd's state in `/var/lib/fwupd` is not kept across boots, and `forge doctor` does not report it.

## `audio`

Sound through PipeWire, with its ALSA and PulseAudio interfaces (32-bit ALSA too on `x86_64-linux`) and rtkit for realtime priority. WirePlumber's state (volumes, default devices) is kept in every home with `wipe-home` ([wipe-home.md](../how-to/wipe-home.md)). With it, Hyprland binds the volume keys ([desktop.md](desktop.md#keybinds)).

## `bluetooth`

Bluetooth on and powered at boot, with BlueZ's experimental features on. Paired devices in `/var/lib/bluetooth` are kept in `/persist`.

## `amd-gpu`

An AMD GPU: the redistributable firmware, the graphics stack, and `amdgpu` loaded from the initrd, before the disk is unlocked.

## `intel-gpu`

An Intel GPU: the redistributable firmware, the graphics stack, and video decoding and encoding through `intel-media-driver` and `vpl-gpu-rt`.

## `nvidia`

An NVIDIA GPU: the proprietary driver with NVIDIA's open kernel modules (Turing and later), its power management for suspend, and the graphics stack. It allows the unfree `nvidia-x11`, `nvidia-settings` and `nvidia-persistenced` packages by name. An older card needs `hardware.nvidia.open = false;` in the host's `module`.

## `raspberry-pi`

The folder publishes two modules, `raspberry-pi-4` and `raspberry-pi-5`, one per board, from [nixos-raspberrypi](https://github.com/nvmd/nixos-raspberrypi): the board's base module and its SD card image, with the vendor kernel and firmware built from this flake's nixpkgs, no ZFS, and the `raspberrypi-eeprom` tools. List one of them with the `pi` profile; [raspberry-pi.md](../how-to/raspberry-pi.md) goes from the host to a flashed card.
