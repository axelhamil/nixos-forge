# Host options

Fields of `configurations.nixos.<host>`, set in `modules/hosts/<host>/default.nix`. Generated from [`modules/flake/hosts/options.nix`](../modules/flake/hosts/options.nix) by `nix run .#options-doc`: do not edit by hand.

## apps

Apps for every user of the machine: an app of ` modules/apps/<name> ` when there is one, else the nixpkgs package of that name (` kdePackages.kate ` works)\. Unfree packages named here are allowed; a wrapper whose unfree part is an inner package (` steam `) still needs that name in ` nixpkgs.config.allowUnfreePackages `\.



*Type:*
list of string



*Default:*

```nix
[ ]
```



*Example:*

```nix
[
  "firefox"
  "vlc"
]
```



## features



NixOS modules the machine turns on, by name: anything a file under ` modules/ ` publishes as ` flake.modules.nixos.<name> ` (` disk `, ` secure-boot `, ` wipe-home `, ` hyprland `, ` plymouth `, ` docker `, ` libvirt `, ` ssh `, ` secrets `, …)\. ` disk ` is the encrypted btrfs layout with ` / ` wiped at boot; the disk it erases is chosen at install (installer ISO, ` disko-install --disk main `, or ` disk.device ` in ` module `), never in the registry\.



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
  "hyprland"
  "docker"
]
```



## hardware



Hardware of the machine, by name: a module of the starter (` nvidia `, ` amd-gpu `, ` intel-gpu `, ` raspberry-pi-4 `, ` raspberry-pi-5 `, …) or a [nixos-hardware](https://github\.com/NixOS/nixos-hardware) profile (` common-pc-ssd `, ` framework-13-7040-amd `, …)\.



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



Role of the machine: ` desktop `, ` laptop `, ` server ` or ` pi ` (` modules/profiles `)\.



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



Accounts of the machine, by login name\. Each one gets home-manager with the ` home ` modules it lists\.



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



Apps for this user only: an app of ` modules/apps/<name> ` when there is one, else the nixpkgs package of that name (` kdePackages.kate ` works)\. Unfree packages named here are allowed\.



*Type:*
list of string



*Default:*

```nix
[ ]
```



*Example:*

```nix
[
  "firefox"
  "obsidian"
]
```



## users\.\<name>\.email



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



## users\.\<name>\.fullName



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



## users\.\<name>\.home



home-manager modules the user imports, by name: the starter’s (` modules/home/<name> `) or your own, such as ` flake.modules.homeManager.<user> ` in ` modules/users/<user> `\.



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
]
```



## users\.\<name>\.sshKeys



Public SSH keys allowed to log in as this user\.



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


