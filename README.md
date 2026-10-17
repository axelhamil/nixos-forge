<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/assets/hero-dark.svg">
    <img src="docs/assets/hero-light.svg" alt="nixos-forge: a doorway drawn in fine lines, its arch keyed by the Nix snowflake between two holly trees" width="100%">
  </picture>
</p>

<p align="center">A NixOS starter you fork and forge: every machine you describe is built, booted and installed in a VM by CI before it reaches yours.</p>

<p align="center"><sub>Fork, friend, and enter.</sub></p>

<div align="center">

<table>
  <tr>
    <td>Proved by CI</td>
    <td>
      <a href="https://github.com/axelhamil/nixos-forge/actions/workflows/check.yml"><img alt="check" src="https://img.shields.io/github/actions/workflow/status/axelhamil/nixos-forge/check.yml?branch=main&style=flat-square&logo=githubactions&logoColor=white&label=check"></a>
      <a href="https://github.com/axelhamil/nixos-forge/actions/workflows/reproducibility.yml"><img alt="reproducibility workflow" src="https://img.shields.io/github/actions/workflow/status/axelhamil/nixos-forge/reproducibility.yml?branch=main&style=flat-square&logo=githubactions&logoColor=white&label=reproducibility"></a>
      <a href="https://github.com/axelhamil/nixos-forge/actions/workflows/update-flake-lock.yml"><img alt="update-flake-lock" src="https://img.shields.io/github/actions/workflow/status/axelhamil/nixos-forge/update-flake-lock.yml?branch=main&style=flat-square&logo=githubactions&logoColor=white&label=update-flake-lock"></a>
      <a href="https://github.com/axelhamil/nixos-forge/actions/workflows/release.yml"><img alt="release" src="https://img.shields.io/github/actions/workflow/status/axelhamil/nixos-forge/release.yml?branch=main&style=flat-square&logo=githubactions&logoColor=white&label=release"></a>
      <br>
      <a href="docs/ci.md#live-badges"><img alt="checks passing" src="https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Faxelhamil%2Fnixos-forge%2Fbadges%2Fchecks.json&style=flat-square&logo=nixos&logoColor=white"></a>
      <a href="docs/ci.md#live-badges"><img alt="VM tests" src="https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Faxelhamil%2Fnixos-forge%2Fbadges%2Fvm-tests.json&style=flat-square&logo=qemu&logoColor=white"></a>
      <a href="docs/ci.md#live-badges"><img alt="hosts built" src="https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Faxelhamil%2Fnixos-forge%2Fbadges%2Fhosts.json&style=flat-square&logo=nixos&logoColor=white"></a>
      <a href="docs/ci.md#live-badges"><img alt="boot time" src="https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Faxelhamil%2Fnixos-forge%2Fbadges%2Fboot-time.json&style=flat-square&logo=nixos&logoColor=white"></a>
      <a href="docs/ci.md#reproducibility"><img alt="reproducibility" src="https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Faxelhamil%2Fnixos-forge%2Fbadges%2Freproducibility.json&style=flat-square&logo=nixos&logoColor=white"></a>
    </td>
  </tr>
  <tr>
    <td>Built on</td>
    <td>
      <a href="https://nixos.org"><img alt="NixOS unstable" src="https://img.shields.io/badge/NixOS-unstable-3F4D66?style=flat-square&logo=nixos&logoColor=white"></a>
      <a href="https://flake.parts"><img alt="flake-parts" src="https://img.shields.io/badge/flake--parts-dendritic-3F4D66?style=flat-square&logo=nixos&logoColor=white"></a>
      <a href="https://github.com/nix-community/home-manager"><img alt="home-manager" src="https://img.shields.io/badge/home--manager-3F4D66?style=flat-square&logo=nixos&logoColor=white"></a>
      <a href="https://github.com/nix-community/disko"><img alt="disko" src="https://img.shields.io/badge/disko-3F4D66?style=flat-square&logo=nixos&logoColor=white"></a>
      <a href="https://github.com/nix-community/lanzaboote"><img alt="lanzaboote" src="https://img.shields.io/badge/lanzaboote-Secure%20Boot-3F4D66?style=flat-square&logo=nixos&logoColor=white"></a>
      <a href="https://github.com/nix-community/impermanence"><img alt="impermanence" src="https://img.shields.io/badge/impermanence-3F4D66?style=flat-square&logo=nixos&logoColor=white"></a>
      <a href="https://github.com/Mic92/sops-nix"><img alt="sops-nix" src="https://img.shields.io/badge/sops--nix-3F4D66?style=flat-square&logo=nixos&logoColor=white"></a>
      <a href="https://hypr.land"><img alt="Hyprland" src="https://img.shields.io/badge/Hyprland-3F4D66?style=flat-square&logo=hyprland&logoColor=white"></a>
    </td>
  </tr>
  <tr>
    <td>Project</td>
    <td>
      <a href="https://github.com/axelhamil/nixos-forge/releases"><img alt="latest release" src="https://img.shields.io/github/v/release/axelhamil/nixos-forge?include_prereleases&sort=semver&style=flat-square&color=6B7A90&label=release"></a>
      <a href="LICENSE"><img alt="license" src="https://img.shields.io/github/license/axelhamil/nixos-forge?style=flat-square&color=6B7A90"></a>
      <a href="https://www.conventionalcommits.org"><img alt="Conventional Commits" src="https://img.shields.io/badge/Conventional%20Commits-1.0.0-6B7A90?style=flat-square&logo=conventionalcommits&logoColor=white"></a>
      <a href="https://github.com/googleapis/release-please"><img alt="release-please" src="https://img.shields.io/badge/release--please-6B7A90?style=flat-square"></a>
      <a href="https://github.com/axelhamil/nixos-forge/commits/develop"><img alt="last commit" src="https://img.shields.io/github/last-commit/axelhamil/nixos-forge/develop?style=flat-square&color=6B7A90"></a>
    </td>
  </tr>
</table>

</div>

In the spirit of [kickstart.nvim](https://github.com/nvim-lua/kickstart.nvim): a readable starting point, not a framework. Every piece is a module you turn on by name, read in one file and drop by deleting it. The commented `example` host lists every profile, feature, hardware module and app, one line each to uncomment, `example` and `example-server` show `wipe-home` and `example-pi` a Raspberry Pi, so you see the whole starter; your fork keeps what it wants.

## What CI proves

Every host in the registry gets its checks generated, so a machine you add is tested without touching YAML. Each run ends in one report per machine, in the run summary and as a single PR comment ([docs/ci.md](docs/ci.md)).

| Check | What it proves |
|---|---|
| `<host>-toplevel` | The system builds with its real revision; this closure is what `auto-upgrade` installs. |
| `<host>-boots` | The machine boots in a VM to `multi-user.target` and every user's home-manager service starts. |
| `<host>-disk` | It installs on a blank encrypted disk, reboots, wipes `/` and takes a snapshot; with `secure-boot`, it boots under Secure Boot and unlocks the disk with the TPM2. |
| `<host>-installer` | The installer ISO installs the host without questions and the installed disk boots. |
| `<host>-wipe-home` | The same install with `wipe-home` added, so `/home` starts empty at every boot. |
| `feature-rollback`, `feature-rollback-secure-boot`, `feature-rollback-failed-unit` | A new generation whose health check fails, or whose service fails after boot, is never blessed: after 3 tries the machine boots the one before. |
| `feature-migration` | A machine installed from the base revision (the PR's base, or the commit before a push) upgrades to this one in place and keeps its state. |
| `ci-promote`, `feature-auto-upgrade-signed` | `deploy/<host>` moves only once every check of that host passed, signed by CI, and a machine applies only a commit signed for it, never an older one replayed. |
| `app-*`, `feature-*`, `profile-*`, `base-*` | Each module with a `check.nix` boots in its own VM test, from the Hyprland session to hibernate and resume. |
| Reproducibility, nightly | Every system is rebuilt with `nix build --rebuild` and compared, with a diffoscope summary when it differs. |
| Boot time | Each boot test records `systemd-analyze`, shown in the report beside the base revision's. |
| Package diff, every PR | [nvd](https://git.sr.ht/~khumba/nvd) lists the packages each system gains, loses or changes. |

## What your fork gets on day one

Fork it, run `nix run .#init`, turn CI on ([docs/ci.md](docs/ci.md#turn-ci-on-in-your-fork)), and your repository works the same way for your machines:

- The same CI on your own hosts: each one is built, booted in a VM and, with `disk`, installed on a blank disk with Secure Boot and TPM2.
- Your own live badges, published by your CI once it runs on `main` ([docs/ci.md](docs/ci.md#live-badges)).
- A pull request every Monday with fresh inputs in `flake.lock`, merged on its own once CI is green.
- A package diff on every pull request.
- Deploy branches signed by your CI with a key `init` generates, which your machines alone accept ([docs/ci.md](docs/ci.md#sign-the-deploy-branches)).
- `auto-upgrade`, which takes only the version CI validated for that machine and on a host with `disk`, goes back to the previous generation when the new one does not boot ([docs/updates.md](docs/updates.md)).
- `nix run .#upstream`, which shows what the starter changed and merges its release when you choose ([docs/updates.md](docs/updates.md#your-fork-follows-the-starter)).

## A machine is one file

```nix
# modules/hosts/my-laptop/default.nix
{
  configurations.nixos.my-laptop = {
    stateVersion = "26.05";
    hardware = [ "framework-13-7040-amd" ];

    profile = "laptop";
    features = [ "disk" "secure-boot" "plymouth" "docker" ];
    apps = [ "hyprland" "firefox" ];

    timeZone = "Europe/Paris";
    keyboard = "fr";

    users.alice = { };
  };
}
```

`users.alice` gives an account to a person defined once, in `modules/users/alice/default.nix`, with her name, email, SSH keys, apps and own home-manager config; every machine that names her gets the same person.

```nix
{
  users.alice = {
    fullName = "Alice Example";
    email = "alice@example.org";
    apps = [ "shell" "stow" ];
  };
}
```

That is the whole host: it gets a `nixosConfigurations` entry, a build check, a VM boot test and, with `disk`, an installer ISO (`nix build .#installer-my-laptop`), with no import list to keep. The disk it erases is picked at install, never written in the host. Every field is in [docs/options.md](docs/options.md), and `module` takes any other NixOS setting.

## What you get

| | |
|---|---|
| **Disk** | [disko](https://github.com/nix-community/disko) layout with LUKS and btrfs, `/` wiped at every boot with [impermanence](https://github.com/nix-community/impermanence), hourly btrbk snapshots, opt-in empty `/home` at every boot, opt-in hibernation to an encrypted swap file. [install](docs/install.md), [wipe-home](docs/wipe-home.md), [hibernate](docs/hibernate.md) |
| **Boot** | opt-in Secure Boot with [lanzaboote](https://github.com/nix-community/lanzaboote); the disk then unlocks with the TPM2, enrolled on its own, passphrase kept as a fallback. [install](docs/install.md) |
| **Install** | `nix build .#installer-<host>` makes an offline ISO of your host: pick a disk, type a passphrase and the passwords, reboot. [install](docs/install.md) |
| **Shell** | zsh, starship, git with delta, neovim, fzf, zoxide, eza, bat, ripgrep, fd, btop through home-manager in the XDG layout; your dotfiles with [stow](https://www.gnu.org/software/stow/). [apps](docs/apps.md) |
| **Desktop** | Plymouth, greetd with tuigreet, [Hyprland](https://hypr.land) with waybar, fuzzel, kitty, mako, hyprlock and hypridle, Firefox with telemetry off. [desktop](docs/desktop.md) |
| **Performance** | `desktop`, `laptop`, `server` and `pi` profiles (kernel, sched-ext, power, zram, oomd). [performance](docs/performance.md) |
| **Hardware** | firmware with CPU microcode, PipeWire, Bluetooth, NVIDIA, AMD and Intel GPUs, any [nixos-hardware](https://github.com/NixOS/nixos-hardware) profile, Raspberry Pi 4 and 5. [raspberry-pi](docs/raspberry-pi.md) |
| **Services and dev** | systemd-resolved, key-only SSH, [sops-nix](https://github.com/Mic92/sops-nix) secrets, [nh](https://github.com/nix-community/nh), offline `nix-locate`, rootless Docker, libvirt, nix-ld, direnv. [services](docs/services.md), [dev](docs/dev.md) |
| **Updates** | every Monday your fork gets a PR with fresh inputs, merged once CI is green; `nix run .#upstream` shows what the starter changed and prepares its merge when you choose; machines pull your fork with `update`, or daily on their own with `auto-upgrade`, which takes only the version CI validated for that machine, and with `signers` only one CI signed for it; a desktop then shows a notification with what changed. [updates](docs/updates.md) |
| **Doctor** | `sudo doctor` checks Secure Boot, the TPM2 token, snapshots, failed units, disk space, your checkout, what the last auto-upgrade did, whether it requires signed deploy commits and whether a boot went back to the previous version, and lists what the next boot erases from `/`. [doctor](docs/doctor.md) |
| **CI** | every host builds and boots in a VM, the install is tested on a blank disk, each PR gets a package diff, `flake.lock` updates itself weekly, only what the cache lacks is built ([above](#what-ci-proves)). [ci](docs/ci.md) |

## Quick start

You need Nix with flakes, and an x86_64 Linux machine with KVM to run the tests and the VM.

1. **Fork and clone** the repo.
2. **Make it yours** with `nix run .#init`: it asks for your host name, profile, login, name, email, time zone, keyboard, locale, features, hardware and apps, writes `modules/hosts/<host>/default.nix` from the commented [example host](modules/hosts/example/default.nix) and `modules/users/<login>/default.nix` from the commented [example person](modules/users/alice/default.nix), removes the examples you do not keep and stages the result. It also generates the SSH key that signs your deploy branches: the public key goes in the `signers` of your hosts, the private key is saved in a file outside the repository, readable by you alone, and init prints the `gh secret set` command that stores it and shreds the file ([ci](docs/ci.md#sign-the-deploy-branches)). In the example hosts you keep, the `flake` they upgrade from becomes your fork's, read from the `origin` remote (`github:<owner>/<repo>` for GitHub, `git+https://...` otherwise); without an `origin` of your own, `flake` and `auto-upgrade` are commented out there, so a kept host never pulls from someone else's repository (`--help` lists a flag for every answer, `--yes` asks nothing). To do it by hand, copy the example host to `modules/hosts/<host>` and the example person to `modules/users/<login>`, rename `example` and `alice`, uncomment what you want, and `git add` them: a flake only sees what git tracks.
3. **Check it and boot it** in a VM:

   ```sh
   nix develop        # installs a pre-push hook that runs in seconds
   nix fmt
   nix flake check -L # ends with: all checks passed!
   nix build .#nixosConfigurations.<host>.config.system.build.vm
   ./result/bin/run-<host>-vm
   ```

   Log in with your user and the password `test`, set for the VM only. Nothing touches a physical disk.
4. **Install** on a real machine: build the installer ISO, write it to a USB stick and boot it; the assistant asks for the disk, the passphrase and the passwords. `nix run .#try-install -- <host>` runs the same install in a local VM first. Other paths (nixos-anywhere, disko-install) are in [docs/install.md](docs/install.md). Turn on the CI of your fork with [docs/ci.md](docs/ci.md).

   ```sh
   nix build .#installer-<host>   # result/iso/<host>-installer.iso
   ```

## Make it yours

| Field | What it takes |
|---|---|
| `profile` | the role: `desktop`, `laptop`, `server` or `pi` |
| `features` | any NixOS module by name: `disk` (the encrypted layout, on the disk picked at install), `secure-boot`, `wipe-home`, `plymouth`, `docker`, `libvirt`, `nix-ld`, `ssh`, `secrets`, yours |
| `apps` | an app of `modules/apps/<name>` with its config (`hyprland`, `direnv`, `firefox`, `stow`, ...), else any nixpkgs package (`kdePackages.kate` works, unfree allowed) |
| `hardware` | a starter module (`nvidia`, `amd-gpu`, `raspberry-pi-5`, ...) or a nixos-hardware profile |
| `users.<name>` | an account for the person of `modules/users/<name>`: `admin`, `groups` and `apps` on this machine, added to the person's |
| `facter` | the hardware report the installer saves in `/persist/facter.json`, copied next to the host file: `./facter.json` |
| `flake` | where the machine updates from: your checkout or `github:<you>/<fork>` |
| `module` | anything else, as plain NixOS config |

A person (`modules/users/<name>`, every field in [docs/options.md](docs/options.md#person-options)) has `fullName`, `email`, `sshKeys`, `apps` that follow them on every machine (`shell` by default) and `module`, their own home-manager config.

An unknown name stops the evaluation with the list of known ones; in `apps`, a name that is no app and no nixpkgs package; in `users`, a person with no folder in `modules/users`.

**Add a feature.** Publish `features.<name>` from a file under `modules/features/<name>` and name it in `features`. Another desktop, Plasma or GNOME, plugs in as an app, a sibling of `hyprland`.

**Add an app with its config** ([docs/apps.md](docs/apps.md)). Create `modules/apps/<name>/default.nix` and name it in `apps`; a `check.nix` with `helpers.appCheck` gives it a VM test, `app-<name>`. [`modules/apps/firefox`](modules/apps/firefox/default.nix) is the example, swap it for the browser you like the same way.

```nix
{
  apps.<name>.homeManager = {
    programs.<name>.enable = true;
  };
}
```

**Add your home config.** `users.<you>.module` in `modules/users/<you>`, home-manager config that follows you on every machine ([docs/apps.md](docs/apps.md#change-a-starter-default)). Every setting a starter app writes, its `enable` flags aside, is a default you override there without `mkForce`.

**Change your system.** Edit the checkout, try it with `nh os test`, switch with `nh os switch`, then commit and push so your other machines follow ([docs/updates.md](docs/updates.md#the-edit-loop)).

## Layout

Every `.nix` file under `modules/` is a [flake-parts](https://flake.parts) module, loaded by [import-tree](https://github.com/vic/import-tree): adding a file adds the feature, deleting it removes it.

```text
modules/
├── flake/     flake wiring: host registry and its checks, installer ISO, formatter, dev shell,
│              CI scripts, helpers/ (functions shared by modules, config.helpers)
├── features/  the system: base/ (nix, home-manager, impermanence, resolved, doctor, update),
│              profiles/ (desktop, laptop, server, pi), hardware/ (firmware, audio, GPUs,
│              Raspberry Pi), and disk, wipe-home, secure-boot, ssh, secrets, docker, libvirt,
│              nix-ld, greetd, plymouth, ...
├── apps/      everything a user runs, with its config: hyprland, waybar, fuzzel, kitty, mako,
│              lock, the shell bundle (zsh, starship, git, neovim, cli), direnv, stow, firefox
├── users/     one folder per person: name, email, SSH keys, apps and home config
└── hosts/     one file per machine, giving accounts to people
```

## Releases

Versions come from Conventional Commits through [release-please](https://github.com/googleapis/release-please). The starter is in alpha: it stays in `0.x` and every release is a pre-release, and your fork takes each one when you run `nix run .#upstream` ([docs/updates.md](docs/updates.md#your-fork-follows-the-starter)). Shipped lots and their CI guardrails are in [ROADMAP.md](ROADMAP.md).

## License

[MIT](LICENSE)

<sub>Why the name: Celebrimbor, the smith of Eregion, forged and taught others to forge their own. You forge your own NixOS from this one. The Nix snowflake in the door comes from [nixos-icons](https://github.com/NixOS/nixos-artwork), under [CC-BY 4.0](https://creativecommons.org/licenses/by/4.0/).</sub>
