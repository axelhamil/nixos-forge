# Dev base

The Nix housekeeping every host gets for free, then the opt-in modules that turn a machine into a dev box: containers, VMs, foreign binaries and per-project shells.

## Every host

**nh** ([nix-community/nh](https://github.com/nix-community/nh)) replaces `nix.gc`: `nh clean all` runs weekly and keeps every generation younger than 14 days plus the last 5; set `programs.nh.clean.extraArgs` in your host to keep other ones. `nh os switch` builds, shows the package diff and switches; `nixos-rebuild` still works. The host's `flake` field points it at your fork and gives the `update` command ([updates.md](../explanation/updates.md)).

Channels are off: `nixpkgs` in `nix shell nixpkgs#hello` and in `NIX_PATH` is the revision the system was built from. User profiles live in `~/.local/state/nix` (`use-xdg-base-directories`). Members of `wheel` are trusted users of the Nix daemon; a host adds users to `nix.settings.trusted-users`, or sets it with `lib.mkForce` to remove `wheel`.

## `nix-index` (opt-in app)

**nix-index-database** ([nix-community/nix-index-database](https://github.com/nix-community/nix-index-database)) gives `nix-locate` a prebuilt database pinned in `flake.lock`. A missing command prints the packages that provide it, and `, <command>` runs it without installing. The `nix-index` app ships the small database (executables only); for full file searches (`nix-locate lib/libssl.so`), set `programs.nix-index.package` to the input's `nix-index-with-db` in your host.

## `docker` (opt-in)

Rootless Docker: the daemon runs as your user, and `docker` and `docker compose` work as usual, with data in `~/.local/share/docker` (in `/persist` on a host with `wipe-home`). It uses the containerd image store, the `local` log driver, and a weekly `docker-prune` timer for stopped containers, unused networks, dangling images and build cache older than a week. `--cpus`, `--cpuset-cpus` and IO limits work.

The daemon stops when you log out, so `--restart` containers only come back at your next login. To keep them running from boot:

```nix
users.users.<you>.linger = true;
```

Gotchas:

- btrbk snapshots `/home` hourly, Docker data included: a prune frees no space until those snapshots expire. To keep Docker out, run `btrfs subvolume create ~/.local/share/docker` before the first start.
- Ports below 1024 need `net.ipv4.ip_unprivileged_port_start`, `--network host` is the user namespace's network, and containers see connections as coming from the host.

## `libvirt` (opt-in)

libvirtd with unprivileged QEMU, UEFI firmware, swtpm (for guests that need a TPM, such as Windows 11), virtiofs, USB redirection and virt-manager. Add your user to `libvirtd` in the host and start the NAT network once:

```nix
users.<you>.groups = [ "libvirtd" ];
```

```sh
virsh -c qemu:///system net-autostart default
virsh -c qemu:///system net-start default
```

`/var/lib/libvirt` is kept in `/persist`. On a btrfs `/`, `/var/lib/libvirt/images` is its own subvolume without copy-on-write (`+C`), so btrbk snapshots skip VM disks: back them up yourself (`virsh snapshot-create`). The attribute only applies to new files: move existing images out and back in.

## `nix-ld` and `direnv` (opt-in)

`nix-ld` is a feature, `direnv` an app: `features = [ "nix-ld" ]; apps = [ "direnv" ];`.

- **`nix-ld`** runs dynamically linked binaries not built for NixOS (editor language servers, prebuilt npm or pip binaries, VS Code server). Add missing libraries to `programs.nix-ld.libraries`. AppImages run by their name too (`./App.AppImage`), through `appimage-run` registered with binfmt.
- **`direnv`** with nix-direnv: `use flake` in a project's `.envrc` loads its dev shell on `cd`, cached.
