# Host options

Fields of `configurations.nixos.<host>`, set in `modules/hosts/<host>/default.nix`. Generated from [`modules/flake/hosts/options.nix`](../../modules/flake/hosts/options.nix) by `nix run .#options-doc`: do not edit by hand.

## apps

Apps for every user of the machine: an app of ` modules/apps/<name> ` with its NixOS and home-manager config when there is one (` hyprland ` is the window manager; its bar, launcher, terminal and lock screen are apps of their own), else the nixpkgs package of that name (` kdePackages.kate ` works)\. Unfree packages named here are allowed; a wrapper whose unfree part is an inner package (` steam `) still needs that name in ` nixpkgs.config.allowUnfreePackages `\.



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



Where the machine updates from: a git checkout on the machine, which the ` update ` command pulls then switches to, or a flake reference such as ` github:<owner>/<repo> `\. The ` auto-upgrade ` feature pulls the ` deploy/<host> ` branch of that fork (of the checkout’s ` origin ` remote)\.



*Type:*
null or string



*Default:*

```nix
null
```



*Example:*

```nix
"/home/<user>/nixos"
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



## mirrors



Extra git URLs of the fork, tried in order when the first one is unreachable: ` https:// `, ` ssh:// `, an scp-like ` user@host:path `, ` file:// ` or an absolute path\. ` http:// ` and ` git:// ` are refused, since they are not encrypted\.



*Type:*
list of string



*Default:*

```nix
[ ]
```



*Example:*

```nix
[
  "https://codeberg.org/<owner>/<repo>.git"
]
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



## signers



SSH public keys that sign the deploy commits CI promotes (the secret ` DEPLOY_SIGNING_KEY `, which ` nix run .#init ` generates)\. When set, ` auto-upgrade ` applies a ` deploy/<host> ` commit only when it carries a valid signature from one of them, and skips a source that serves anything else\. Empty, it trusts whatever its sources serve, and ` doctor ` warns\.



*Type:*
list of string



*Default:*

```nix
[ ]
```



*Example:*

```nix
[
  "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIExampleKeyOfTheDeploySecret deploy"
]
```



## specialisations



Variants of the machine, by name, each one the whole host plus its own ` features `, ` apps `, ` hardware ` and ` module `\. Each variant is an entry of the boot menu, and ` update -s <name> ` switches to it without a reboot (` update -S ` goes back); a kernel or kernel module it changes takes effect only when it is started from the boot menu\. The running variant’s name is in ` /etc/specialisation `, and ` update ` and ` auto-upgrade ` keep it\.



*Type:*
attribute set of (submodule)



*Default:*

```nix
{ }
```



*Example:*

```nix
{ gaming.apps = [ "steam" "gamemode" ]; }
```



## specialisations\.\<name>\.apps



Apps this variant adds to the host’s ` apps ` for every user, by the same names: an app of ` modules/apps/<name> `, else a nixpkgs package\.



*Type:*
list of string



*Default:*

```nix
[ ]
```



*Example:*

```nix
[
  "steam"
  "gamemode"
  "mangohud"
]
```



## specialisations\.\<name>\.features



Features this variant adds to the host’s ` features `, by the same names\.



*Type:*
list of string



*Default:*

```nix
[ ]
```



*Example:*

```nix
[
  "libvirt"
]
```



## specialisations\.\<name>\.hardware



Hardware modules this variant adds to the host’s ` hardware `, by the same names\.



*Type:*
list of string



*Default:*

```nix
[ ]
```



*Example:*

```nix
[
  "nvidia"
]
```



## specialisations\.\<name>\.module



Any other NixOS configuration of this variant, on top of the host’s\.



*Type:*
module



*Default:*

```nix
{ }
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

Fields of `users.<person>`, set in `modules/users/<person>/default.nix`: the person a host gives an account with `users.<person>`. Generated from [`modules/flake/hosts/users.nix`](../../modules/flake/hosts/users.nix).

## apps

Apps that follow the person on every host, none by default: an app of ` modules/apps/<name> ` with its NixOS and home-manager config when there is one (` zsh ` also becomes the login shell), else the nixpkgs package of that name (` kdePackages.kate ` works)\. Unfree packages named here are allowed\. A host adds its own with ` users.<name>.apps `\.



*Type:*
list of string



*Default:*

```nix
[ ]
```



*Example:*

```nix
[
  "zsh"
  "git"
  "neovim"
  "ripgrep"
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
"you@example.org"
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
"Your Name"
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
  "ssh-ed25519 AAAA... <user>@laptop"
]
```



## u2fKeys



Security keys that log the person in and run ` sudo ` without a password, on every host with the ` fido2 ` feature, one line of ` pamu2fcfg -n -o pam://nixos-forge -i pam://nixos-forge ` each\. Without a key plugged in, the password still works\.



*Type:*
list of string matching the pattern :?\[^:
]+



*Default:*

```nix
[ ]
```



*Example:*

```nix
[
  ":owBYz...,pSJ7...,es256,+presence"
]
```


