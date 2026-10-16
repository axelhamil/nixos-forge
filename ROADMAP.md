# Roadmap

Each lot builds and boots on its own before the next starts, and keeps a guardrail in CI: a change that touches a lot keeps its checks green. Anything beyond the shared base ships as an opt-in module with neutral defaults; personal choices belong in forks. Everything is developed and tested in VMs.

## Shipped

| Lot | Guardrail |
|---|---|
| **1. Repo tooling**: dendritic flake, nixfmt, statix, deadnix, CI matrix with KVM, PR package diff, weekly lock update, release-please | `treefmt`, `ci-ok` required on `main` |
| **2. Host registry**: `configurations.nixos.<name>` generates the config, its checks and its CI entry | `<host>-toplevel`, `<host>-boots` |
| **3. Disk**: disko layout, LUKS2, btrfs subvolumes, `/` rolled back at every boot, state declared in `/persist` | `<host>-disk` |
| **4. Boot**: opt-in `secure-boot` with lanzaboote, TPM2 unlock enrolled on its own (PCR 7 and 15), passphrase kept | `<host>-disk` on a `secure-boot` host |
| **5. Hardware**: `firmware`, `audio`, `bluetooth`, `nvidia`, `amd-gpu`, `intel-gpu`, nixos-hardware profiles, unfree packages allowed by name | `<host>-toplevel` of hosts importing them |
| **6. Performance profiles**: `desktop`, `laptop`, `server` (kernel, sched-ext, power, zram, oomd) | `profile-desktop`, `profile-laptop`, `profile-server` |
| **7. Services**: systemd-resolved, btrbk snapshots, opt-in `ssh` and `secrets` (sops-nix) | `system-resolved`, `system-ssh`, `system-secrets`, `<host>-disk` |
| **8. Dev base**: `nh clean`, nix-index with comma, opt-in rootless `docker`, `libvirt`, `nix-ld`, `direnv` | `system-nix`, `system-nix-index`, `dev-docker`, `dev-libvirt` |
| **9a. Shell**: home-manager `home/` layer (zsh, starship, git, neovim, CLI tools), XDG layout, opt-in `stow` | `home-shell`, `home-stow` |
| **9b. Wipe `/home`**: opt-in `wipe-home`, previous home kept 7 days, declared paths restored from `/persist` | `<host>-wipe-home` |
| **10. Desktop**: `greetd`, `plymouth`, `hyprland` with waybar, fuzzel, kitty, mako, hyprlock and hypridle | `desktop-hyprland`, `desktop-plymouth` |
| **11. Raspberry Pi**: `raspberry-pi-4` and `raspberry-pi-5` on nixos-raspberrypi, `pi` profile, SD image | `<host>-sd-image`, `profile-pi` |
| **12. One file per machine**: registry fields in plain values, users generated from the host, `apps/` layer with `firefox`, generated `docs/options.md` | `app-firefox`, `options-doc` |
| **13. Fast CI**: only checks missing from the binary cache are built, no kernel built in a PR | `ci-plan` |
| **14. Installer ISO**: `nix build .#installer-<host>` for every host with `disk`, `install-host` assistant on tty1 | `example-server-installer` |
| **15. Updates**: host `flake` field, `update` command, opt-in weekly `auto-upgrade`, weekly `sync-upstream` PR in forks, merged by hand | `system-update` |
| **16. Hardware report**: host `facter` field reading the installer's nixos-facter report | `facter` |
