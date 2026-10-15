# nixos

Config NixOS déclarative, réécrite from scratch, en pattern dendritic (flake-parts + import-tree) : chaque fichier de `modules/` est un module flake-parts. `modules/` est rangé en couches, chaque outil a son dossier dans sa couche (`default.nix`, puis d’autres modules, scripts ou assets au besoin), jamais de fichier à plat :

- `flake/` : câblage du flake (flake-parts, systèmes)
- `system/` : socle OS (nix, locale, boot, réseau)
- `desktop/` : session graphique (hyprland, barre, launcher)
- `apps/` : outils et applis (terminal, shell, éditeur)
- `users/` : comptes et leur home-manager
- `hosts/` : une machine = la liste des modules qu’elle assemble

Repo indépendant des dotfiles Arch : rien ici ne touche à la machine actuelle.

## Tester depuis Arch

```sh
nix build .#nixosConfigurations.desktop.config.system.build.vm
./result/bin/run-desktop-vm
```

Utilisateur `axel`, mot de passe `test` (VM uniquement).

## Plan

1. Socle bootable en VM
2. Desktop Hyprland, terminal, shell, outils dev
3. NVIDIA, gaming, virtualisation
4. Install réelle en dual boot à côté d'Arch
5. Secure Boot (lanzaboote) et entrée Windows dans systemd-boot
6. Transfert des données, suppression d'Arch
