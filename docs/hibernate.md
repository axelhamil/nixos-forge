# Hibernate

Close the lid, walk away, come back to the same session days later. A host with `disk` that adds `hibernate` to its features can hibernate, and closing the lid suspends, then hibernates after a while. This page covers what it sets up, how to add it to a machine already installed, and how it lives with Secure Boot.

```nix
features = [ "disk" "hibernate" ];
module.hibernate.swapSize = "32G";
```

## What it sets

| | |
|---|---|
| Swap | a `@swap` btrfs subvolume on `/swap`, inside LUKS, holding `/swap/swapfile` of `hibernate.swapSize` (`16G` by default: match the RAM). Not snapshotted by btrbk, not touched by the rollback of `/`. |
| zram | stays first: its priority (5) is above the swap file's (0), so the swap file only fills when zram is full, and at hibernation. |
| Resume | the systemd initrd resumes from the `HibernateLocation` EFI variable that systemd writes at hibernation: no `resume=` nor `resume_offset` on the kernel command line, so nothing to compute after the install. |
| Lid | `HandleLidSwitch = "suspend-then-hibernate"` (a default you can override), hibernating after `hibernate.delay` (`2h`) asleep. |

At resume the initrd unlocks the disk first (passphrase, or the TPM2 when enrolled), then systemd-hibernate-resume restores the image. The rollback of `/` (and of `/home` with `wipe-home`) runs after it, so it never touches a hibernated system: a resume keeps `/` as it was, and only a normal boot wipes it. The module asserts this ordering, and the `feature-hibernate` check installs a machine, hibernates it, resumes it with `/` and its processes intact, then boots it normally and finds `/` wiped.

## Add it to an installed machine

disko creates the swap file at install only. On a machine installed without it, create it by hand, then add the feature and switch:

```sh
sudo mount -o subvol=/ /dev/mapper/cryptroot /mnt
sudo btrfs subvolume create /mnt/@swap
sudo btrfs filesystem mkswapfile --size 16G /mnt/@swap/swapfile
sudo umount /mnt
```

Changing `hibernate.swapSize` later also takes a new swap file: `swapoff /swap/swapfile`, delete it, `mkswapfile` again, `swapon`.

## Secure Boot and the TPM2

Linux refuses to hibernate under kernel lockdown, since nothing signs the image. NixOS kernels are built without the lockdown module (`SECURITY_LOCKDOWN_LSM` off) and lanzaboote does not turn it on, so hibernation works with `secure-boot`. A kernel built with that module and booted with `lockdown=integrity` or `confidentiality` loses it.

The image sits inside LUKS, so it is encrypted at rest. With a TPM2 token, the disk opens on its own at resume like at boot: the session is then guarded by the screen lock, which the `lock` app turns on before sleeping when the host or the person lists it.
