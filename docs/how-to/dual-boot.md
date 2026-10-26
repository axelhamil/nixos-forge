# Dual boot with Windows (`dual-boot`)

A host with `disk` that adds `dual-boot` to its features shows Windows in the systemd-boot menu. Power on, pick NixOS or Windows, and both start with Secure Boot on. You never open the firmware menu.

```nix
features = [ "disk" "secure-boot" "dual-boot" ];
module.dualBoot.windowsPartition = "/dev/disk/by-uuid/0123456789ABCDEF";
```

It works with and without `secure-boot`, with Windows on its own disk or on the same disk as NixOS.

The ESP (EFI System Partition) is the small FAT partition the firmware reads boot loaders from. BitLocker is Windows' disk encryption, unlocked by the TPM when the boot chain looks unchanged.

## Windows on its own disk

Install NixOS on its disk as usual ([install.md](../tutorials/install.md)); Windows keeps its disk.

With `secure-boot`, first suspend BitLocker in Windows (Control Panel, BitLocker, Suspend protection) and keep the recovery key at hand. Setup mode and the new keys change what the TPM measures, so BitLocker would ask for its recovery key at the next start.

At every boot, the `dual-boot` service:

1. looks for another ESP holding `EFI/Microsoft/Boot/bootmgfw.efi` and mounts it read-only in a private directory;
2. copies that file onto the NixOS ESP when it is missing or differs, so a Windows update of its boot manager is followed;
3. removes its copy when no disk holds Windows any more.

The copy only makes the menu entry appear. `BootNext` makes the firmware start the original on the Windows disk, and nothing is ever written to that disk.

The service replaces or removes only a file it copied itself, marked by `EFI/Microsoft/Boot/copied-by-nixos`. A boot manager Windows installed on the NixOS ESP (the same-disk case) is never touched, even with another Windows disk plugged in.

## Windows on the same disk

NixOS goes in the free space of the disk and shares the Windows ESP. Before the install, in Windows:

1. Turn Fast Startup off (Control Panel, Power Options, Choose what the power buttons do), so Windows really shuts down.
2. Shrink `C:` in Disk Management (Shrink Volume) to leave the free space NixOS gets: 64 GB or more.
3. Check the ESP size (Disk Management, the `EFI System Partition`). Windows makes it 100 MB, too small: the install needs room for 3 generations of the kernel and initrd. Grow it to 1 GiB first. Shrink `C:` 1 GiB more, then from a live USB move the partitions after the ESP with GParted and grow the ESP. Otherwise put NixOS on its own disk.
4. Suspend BitLocker if it is on (Control Panel, BitLocker, Suspend protection) and keep the recovery key at hand. With `secure-boot`, enrolling the new keys changes what the TPM measures.

Then boot the installer ISO of the host and run:

```bash
forge install --beside-windows
```

`forge install` refuses `--beside-windows` for a host without `dual-boot`. It runs `forge dual-boot install-beside-windows`, which:

- finds the single ESP of the disk and checks that Windows is installed on it;
- checks the ESP free space, and stops with how to grow it when it is too small; it never truncates anything;
- creates the NixOS partition (LUKS, then the usual btrfs subvolumes, [install.md](../tutorials/install.md)) in the largest free block, or reuses the one of a previous install;
- names the ESP `disk-main-ESP` in the partition table, so the installed system mounts it on `/boot` by its label like any host. That GPT name is the only change to a Windows partition entry, and Windows does not read it;
- formats and mounts only the NixOS partition, and mounts the ESP without formatting it.

The MSR, `C:` and the recovery partition stay byte for byte as they were, which `feature-dual-boot-same-disk` checks. The shared ESP does change:

- its GPT name;
- the files lanzaboote and systemd-boot write in `EFI/nixos`, `EFI/Linux`, `EFI/systemd`, `loader` and `EFI/BOOT`;
- the `BOOTX64.EFI` fallback, replaced with systemd-boot.

`EFI/Microsoft` stays Windows'.

`dual-boot` keeps 3 generations on the ESP, the number the install checks room for. It sets `boot.loader.systemd-boot.configurationLimit` with `mkDefault`, and lanzaboote follows it. Raise it only on an ESP with room to spare. A boot loader install that runs out of room fails the switch instead of passing silently.

## Secure Boot keys

`secure-boot` enrolls its own keys alongside Microsoft's ([install.md](../tutorials/install.md#step-2-put-the-firmware-in-setup-mode)). Microsoft's keys stay, so the firmware keeps trusting Windows' boot manager and both systems start with Secure Boot on. Never enroll its keys alone on a dual-boot machine: Windows would no longer start.

## Check it worked

On NixOS, after the first boot with `dual-boot`:

```sh
bootctl list | grep auto-windows   # the menu has a Windows entry
efibootmgr                         # BootOrder: Linux Boot Manager first, Windows Boot Manager second
journalctl -b -u dual-boot         # what the service found and changed
```

Then reboot and pick Windows in the menu. It starts without asking for its BitLocker recovery key. On a host with `secure-boot`, `msinfo32` in Windows shows Secure Boot State `On`.

## How Windows starts

systemd-boot lists Windows as the entry `auto-windows` when `EFI/Microsoft/Boot/bootmgfw.efi` is on its own ESP. `dual-boot` turns on systemd-boot's `reboot-for-bitlocker`. Picking Windows then works like this:

1. systemd-boot does not run Microsoft's boot manager itself.
2. It sets the firmware's `BootNext` to the firmware entry named `Windows Boot Manager`, and restarts.
3. The firmware starts Windows itself, so the Secure Boot chain and the TPM measurements are those of a machine without Linux.
4. BitLocker unlocks without its recovery key.

systemd-boot (systemd 261) takes that path only when the machine has a TPM, a BitLocker volume and a `Windows Boot Manager` entry in `BootOrder`. Otherwise it gives the entry up, and the firmware goes on to the next entry of its boot order.

The `dual-boot` service keeps that order right, at every boot and after every switch: `Linux Boot Manager` first, `Windows Boot Manager` second, the rest after. So without BitLocker the firmware still starts Windows next, never its own menu. When the firmware lost one of the two entries (a firmware update, a reset of its settings), the service creates it again at the next boot of NixOS.

## Reading Windows from NixOS

`ntfs3` is the kernel driver for NTFS. A desktop file manager mounts the Windows partition read-only on a click, through udisks2.

With `dualBoot.windowsPartition` set, it is also mounted read-only on `dualBoot.windowsMount` (`/mnt/windows`) when first read. A missing partition never blocks the boot. Find its UUID or label with `lsblk -o NAME,FSTYPE,LABEL,UUID`.

Read-only is on purpose. Windows' Fast Startup and hibernation leave NTFS in a state where a write from Linux corrupts it. To write anyway, turn Fast Startup off, shut Windows down fully, and mount it read-write by hand (`udisksctl mount -b /dev/<partition> -o rw`).

A partition encrypted with BitLocker opens with its recovery key (Windows: `manage-bde -protectors -get C:`):

```bash
sudo cryptsetup bitlkOpen /dev/<partition> windows
sudo mount -t ntfs3 -o ro /dev/mapper/windows /mnt/windows
sudo umount /mnt/windows && sudo cryptsetup close windows
```

`cryptsetup` cannot create a BitLocker volume, so no check covers this step.

Windows reading NixOS is out of scope: NixOS lives in LUKS.

## Troubleshooting

| Symptom | Fix |
|---|---|
| No Windows in the menu | `journalctl -b -u dual-boot`: the service says what it found. Same disk: `ls /boot/EFI/Microsoft/Boot/bootmgfw.efi`. Separate disk: the other disk must have an ESP of type `EF00` holding that file. |
| Windows in the menu, but picking it boots NixOS again | The firmware has no `Windows Boot Manager` entry: `efibootmgr` lists them. The next boot of NixOS creates it; `sudo systemctl restart dual-boot` creates it now. |
| The machine starts Windows directly, without the menu | A Windows update put its entry first. Boot NixOS once from the firmware boot menu: the `dual-boot` service puts NixOS first again. |
| BitLocker asks for its recovery key | Something else than the firmware started Windows, or the Secure Boot keys changed (a reinstall of NixOS enrolls new ones). Type the key once, then let Windows reseal BitLocker by suspending and resuming protection. |
| The install stops on the ESP size | Grow the ESP as in [Windows on the same disk](#windows-on-the-same-disk), or install on another disk. |
| `/mnt/windows` is empty or refuses to mount | Windows was hibernated or left with Fast Startup: start Windows, turn Fast Startup off, shut down, try again. `journalctl -u mnt-windows.mount` says why. |
| The Windows disk was removed | The next boot removes the copy of its boot manager, and the menu stops showing Windows. |
