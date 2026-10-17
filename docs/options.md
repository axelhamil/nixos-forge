# Host options

Fields of `configurations.nixos.<host>`, set in `modules/hosts/<host>/default.nix`. Generated from [`modules/flake/hosts/options.nix`](../modules/flake/hosts/options.nix) by `nix run .#options-doc`: do not edit by hand.

## apps

Apps for every user of the machine: an app of ` modules/apps/<name> ` with its NixOS and home-manager config when there is one (` hyprland ` is the desktop), else the nixpkgs package of that name (` kdePackages.kate ` works)\. Unfree packages named here are allowed; a wrapper whose unfree part is an inner package (` steam `) still needs that name in ` nixpkgs.config.allowUnfreePackages `\.



*Type:*
list of string



*Default:*

```nix
[ ]
```



*Example:*

```nix
[
  "hyprland"
  "firefox"
  "vlc"
]
```



## facter



Hardware report of the machine from nixos-facter, which the installer ISO saves in ` /persist/facter.json `\. Copy it next to the host file: nixpkgs’ facter module then turns on the kernel modules, graphics, network and virtualisation support that machine needs\.



*Type:*
null or absolute path



*Default:*

```nix
null
```



*Example:*

```nix
./facter.json
```



## features



NixOS modules the machine turns on, by name: the features of ` modules/features/<name> ` (` disk `, ` secure-boot `, ` wipe-home `, ` plymouth `, ` docker `, ` libvirt `, ` ssh `, ` secrets `, …)\. ` disk ` is the encrypted btrfs layout with ` / ` wiped at boot; the disk it erases is chosen at install (installer ISO, ` disko-install --disk main `, or ` disk.device ` in ` module `), never in the registry\.



*Type:*
list of string



*Default:*

```nix
[ ]
```



*Example:*

```nix
[
  "disk"
  "secure-boot"
  "docker"
]
```



## flake



Where the machine updates from: a git checkout on the machine, which the ` update ` command pulls then switches to, or a flake reference such as ` github:alice/nixos `, which the ` auto-upgrade ` feature needs\.



*Type:*
null or string



*Default:*

```nix
null
```



*Example:*

```nix
"/home/alice/nixos"
```



## hardware



Hardware of the machine, by name: a module of ` modules/features/hardware/<name> ` (` nvidia `, ` amd-gpu `, ` intel-gpu `, ` raspberry-pi-4 `, ` raspberry-pi-5 `, …) or a [nixos-hardware](https://github\.com/NixOS/nixos-hardware) profile (` common-pc-ssd `, ` framework-13-7040-amd `, …)\.



*Type:*
list of string



*Default:*

```nix
[ ]
```



*Example:*

```nix
[
  "common-pc-ssd"
  "common-cpu-intel"
]
```



## keyboard



Keyboard layout, for the console and the desktop\.



*Type:*
string



*Default:*

```nix
"us"
```



*Example:*

```nix
"fr"
```



## locale



Language and formats\.



*Type:*
string



*Default:*

```nix
"en_US.UTF-8"
```



*Example:*

```nix
"fr_FR.UTF-8"
```



## module



Any other NixOS configuration of the machine\. The fields above are written into it\.



*Type:*
module



*Default:*

```nix
{ }
```



## profile



Role of the machine: ` desktop `, ` laptop `, ` server ` or ` pi `, the profiles of ` modules/features/profiles/<name> `\.



*Type:*
null or string



*Default:*

```nix
null
```



*Example:*

```nix
"laptop"
```



## stateVersion



NixOS release the machine was first installed with\. Set it once and never change it\.



*Type:*
string



*Example:*

```nix
"26.05"
```



## system



Platform the host runs on; its checks run on a CI runner of the same platform\.



*Type:*
one of “x86_64-linux”, “aarch64-linux”



*Default:*

```nix
"x86_64-linux"
```



## testScript



Python lines appended to the host boot test, run against the ` machine ` node\.



*Type:*
strings concatenated with “\\n”



*Default:*

```nix
""
```



## timeZone



Time zone\.



*Type:*
string



*Default:*

```nix
"UTC"
```



*Example:*

```nix
"Europe/Paris"
```



## users



Accounts of the machine, by login name: each one is a person of ` modules/users/<name> ` (the ` users ` of the flake), with their identity, apps and home-manager ` module `, plus the host’s ` apps `\. The fields below are what changes from one machine to another\.



*Type:*
attribute set of (submodule)



*Default:*

```nix
{ }
```



## users\.\<name>\.admin



Whether the user may run ` sudo ` (member of ` wheel `)\.



*Type:*
boolean



*Default:*

```nix
true
```



## users\.\<name>\.apps



Apps for this user on this machine only, added to the person’s ` apps `: an app of ` modules/apps/<name> `, else the nixpkgs package of that name\. Unfree packages named here are allowed\.



*Type:*
list of string



*Default:*

```nix
[ ]
```



*Example:*

```nix
[
  "stow"
  "kdePackages.kate"
]
```



## users\.\<name>\.groups



Extra groups of the user, besides ` wheel ` (from ` admin `) and ` networkmanager ` (when NetworkManager is on)\.



*Type:*
list of string



*Default:*

```nix
[ ]
```



*Example:*

```nix
[
  "libvirtd"
]
```



# Person options

Fields of `users.<person>`, set in `modules/users/<person>/default.nix`: the person a host gives an account with `users.<person>`. Generated from [`modules/flake/hosts/users.nix`](../modules/flake/hosts/users.nix).

## apps

Apps that follow the person on every host: an app of ` modules/apps/<name> ` with its NixOS and home-manager config when there is one (` shell `, the default, brings zsh as login shell, starship, git, neovim and CLI tools), else the nixpkgs package of that name (` kdePackages.kate ` works)\. Unfree packages named here are allowed\. A host adds its own with ` users.<name>.apps `\.



*Type:*
list of string



*Default:*

```nix
[
  "shell"
]
```



*Example:*

```nix
[
  "shell"
  "stow"
  "obsidian"
]
```



## email



Email used as git author email\.



*Type:*
null or string



*Default:*

```nix
null
```



*Example:*

```nix
"alice@example.org"
```



## fullName



Full name, shown by the greeter and used as git author name\.



*Type:*
string



*Default:*

```nix
""
```



*Example:*

```nix
"Alice Example"
```



## module



The person’s own home-manager config, on every host that gives them an account\. Several files under ` modules/users/<name> ` may each set part of it\.



*Type:*
module



*Default:*

```nix
{ }
```



*Example:*

```nix
{ pkgs, ... }:
{
  home.packages = [ pkgs.gimp ];
  programs.git.settings.alias.st = "status";
}

```



## sshKeys



Public SSH keys allowed to log in as this person, on every host that gives them an account\.



*Type:*
list of string



*Default:*

```nix
[ ]
```



*Example:*

```nix
[
  "ssh-ed25519 AAAA... alice@laptop"
]
```


