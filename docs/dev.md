# Dev base

What every host gets to manage Nix itself, and the opt-in modules for development machines. Nothing here calls a third-party service at runtime.

## Every host

**nh** ([nix-community/nh](https://github.com/nix-community/nh)) replaces `nix.gc`. `nh clean all` runs weekly (caught up at the next boot if the machine was off) and keeps every generation younger than 14 days plus the last 5, across system, home-manager and user profiles. It also drops `result` links untouched for 14 days, but keeps the latest dev shell of every direnv project, so an old project opens without rebuilding. Channels are off: `nixpkgs` in `nix shell nixpkgs#hello` and in `NIX_PATH` is the revision the system was built from. Nix keeps user profiles in `~/.local/state/nix` instead of `~/.nix-profile` and `~/.nix-defexpr` (`use-xdg-base-directories`). `nh os switch` builds, shows the package diff and switches; `nixos-rebuild` still works. Point it at your clone once in your host so it needs no argument:

```nix
programs.nh.flake = "/home/<you>/nixos";
```

`system-nix` checks the `nh clean` timer replaces `nix.gc`.

**nix-index-database** ([nix-community/nix-index-database](https://github.com/nix-community/nix-index-database)) gives `nix-locate` a prebuilt database pinned in `flake.lock`, so it answers offline and needs no hour-long local index. A missing command prints the packages that provide it, and `, <command>` runs it without installing anything. The starter ships the small database (executables only, about 2 MB instead of 100 MB): that is all command-not-found and comma read. For full file searches (`nix-locate lib/libssl.so`), set `programs.nix-index.package` to `nix-index-with-db` in your host. The weekly lock update refreshes it with nixpkgs.

`system-nix-index` checks `nix-locate` finds `hello` offline and that `,` is installed.

## `docker` (opt-in)

Rootless Docker: the daemon runs as your user, so a container escape gets your rights, not root's (members of the classic `docker` group are root-equivalent). `DOCKER_HOST` points the CLI at it, so `docker` and `docker compose` work as usual, with data in `~/.local/share/docker` (kept with `/home`, or in `/persist/home/<you>` on a host with `wipe-home`). The daemon only starts for members of `users` (the default group of every normal user), not for system accounts.

- **containerd image store** with its `overlayfs` snapshotter, even on btrfs and on a data directory created by an older Docker: multi-platform images and attestations work, and nothing falls back to the legacy btrfs graph driver.
- **`local` log driver**: container logs rotate instead of growing forever.
- **Weekly prune** of stopped containers, unused networks, dangling images and build cache older than a week (`docker-prune.timer`, per user, caught up after the machine was off).
- **CPU, cpuset, IO, memory and pids controllers delegated** to user sessions, so `--cpus`, `--cpuset-cpus` and IO limits work rootless.

The daemon lives in your user session: it stops when you log out, and containers with `--restart` only come back at your next login. To keep them running from boot, enable lingering:

```nix
users.users.<you>.linger = true;
```

btrbk snapshots `/home` every hour, images and volumes included: a prune frees no space until those snapshots expire. To keep Docker out of the snapshots, make its directory a subvolume before the first start: `btrfs subvolume create ~/.local/share/docker`. On a host with `wipe-home`, the data sits in `/persist`, which btrbk snapshots the same way.

Rootless limits: ports below 1024 need `net.ipv4.ip_unprivileged_port_start`, `--network host` is the user namespace's network, not the machine's, and containers see connections as coming from the host, not from the client's address.

`dev-docker` runs a container as a user on a btrfs `/home`, and checks the daemon is rootless, stores images with containerd's `overlayfs` snapshotter, rotates logs and schedules a persistent prune.

## `libvirt` (opt-in)

libvirtd with QEMU running unprivileged (`runAsRoot = false`), UEFI firmware from QEMU, swtpm for guests that need a TPM (Windows 11), virtiofs for shared folders, USB redirection and virt-manager. Add your user to the `libvirtd` group to manage VMs without sudo, and start the NAT network once so guests get internet (`virbr0` is a trusted firewall interface, so guests reach libvirt's DHCP and DNS):

```nix
users.users.<you>.extraGroups = [ "libvirtd" ];
```

```sh
virsh -c qemu:///system net-autostart default
virsh -c qemu:///system net-start default
```

`/var/lib/libvirt` and the swtpm certificate authority (`/var/lib/swtpm-localca`) are kept in `/persist`; swtpm runs as root, since the starter creates no `tss` user. On hosts whose `/` is a btrfs subvolume (every host with `disk`), `images` is its own subvolume with the `C` attribute (no copy-on-write): VM disks rewritten in place do not fragment, and btrbk's hourly snapshots of `/persist` skip them (a snapshot would bring copy-on-write back). Snapshot or back up a VM yourself (`virsh snapshot-create`). The attribute only applies to files created afterwards: move existing images out and back in. On other hosts `images` is a plain directory.

`dev-libvirt` boots on a btrfs `/` and checks libvirtd answers, `images` is a `nodatacow` subvolume and guests get an emulated TPM.

## `nix-ld` and `direnv` (opt-in)

- **`nix-ld`** runs dynamically linked binaries not built for NixOS (language servers downloaded by an editor, prebuilt npm or pip binaries, VS Code server). Add libraries a binary misses to `programs.nix-ld.libraries`.
- **`direnv`** comes with nix-direnv: `use flake` in a project's `.envrc` loads its dev shell when you `cd` in, cached so it does not re-evaluate every time.
