# Changelog

## [0.2.5](https://github.com/axelhamil/nixos-forge/compare/v0.2.4...v0.2.5) (2026-10-27)


### Features

* **apps:** port stow-dotfiles and upgrade-notify to rust ([#105](https://github.com/axelhamil/nixos-forge/issues/105)) ([b363377](https://github.com/axelhamil/nixos-forge/commit/b36337729c9214289256e6faa70f19654c1495e8))
* **doctor:** replace doctor.sh with forge doctor and let forks add checks ([#97](https://github.com/axelhamil/nixos-forge/issues/97)) ([4e9afe4](https://github.com/axelhamil/nixos-forge/commit/4e9afe425249aab5b5d6c1b35ac2535095ef5e7a))
* **forge-ci:** port the ci scripts and the report to a rust binary ([#104](https://github.com/axelhamil/nixos-forge/issues/104)) ([71039c8](https://github.com/axelhamil/nixos-forge/commit/71039c87370a7e69cbefe9fea2e25a23014ed152))
* **forge:** add the forge cli in rust and port check-docs to it ([#96](https://github.com/axelhamil/nixos-forge/issues/96)) ([26afb92](https://github.com/axelhamil/nixos-forge/commit/26afb92ce66cd8f9ce7b25da96f021077f1645cc))
* **forge:** port init.sh and upstream.sh to forge init and forge upstream ([#103](https://github.com/axelhamil/nixos-forge/issues/103)) ([bd5d8e6](https://github.com/axelhamil/nixos-forge/commit/bd5d8e691b5f114540ebf5299fc95adf542a4067))
* **forge:** port install-host and the enroll scripts to forge install and forge enroll ([#99](https://github.com/axelhamil/nixos-forge/issues/99)) ([e90017c](https://github.com/axelhamil/nixos-forge/commit/e90017cbeddcfd8efc49991c298a53eff618166a))
* **forge:** port the dual-boot scripts to forge dual-boot ([#102](https://github.com/axelhamil/nixos-forge/issues/102)) ([d58dc75](https://github.com/axelhamil/nixos-forge/commit/d58dc7579f331c01a5a62f8809876cd407b1e014))
* **forge:** port update and auto-upgrade to forge update and forge auto-upgrade ([#98](https://github.com/axelhamil/nixos-forge/issues/98)) ([9fea0ed](https://github.com/axelhamil/nixos-forge/commit/9fea0edd743141f007a3ddbffba735da5051a5c7))


### Bug fixes

* **ci:** fetch each kernel from cachix before pinning it ([#92](https://github.com/axelhamil/nixos-forge/issues/92)) ([ab58e67](https://github.com/axelhamil/nixos-forge/commit/ab58e678dbc4cc8b7eeff706ac314d893853a7c1))
* **ci:** fetch the references of a kernel output before pinning it ([#94](https://github.com/axelhamil/nixos-forge/issues/94)) ([f58f360](https://github.com/axelhamil/nixos-forge/commit/f58f36067b49a1b994c91f057ead3d003929e7c1))
* **ci:** keep every host kernel cached by pinning it in cachix ([#91](https://github.com/axelhamil/nixos-forge/issues/91)) ([f6081e6](https://github.com/axelhamil/nixos-forge/commit/f6081e672835e01471fb92e4de75c3d6b07e6ea4))
* **ci:** realise a kernel output from every substituter before pinning it ([#95](https://github.com/axelhamil/nixos-forge/issues/95)) ([3aff570](https://github.com/axelhamil/nixos-forge/commit/3aff5705a43728c575c7eb52af526d11d3016924))
* **doctor:** never open a pruned mount point, so automounts stay idle ([#101](https://github.com/axelhamil/nixos-forge/issues/101)) ([3077125](https://github.com/axelhamil/nixos-forge/commit/3077125ac6bae1841984402498c46705ec8fe905))

## [0.2.4](https://github.com/axelhamil/nixos-forge/compare/v0.2.3...v0.2.4) (2026-10-22)


### Bug fixes

* **ci:** let the flaky plan evaluate every vm check within the runner ([#88](https://github.com/axelhamil/nixos-forge/issues/88)) ([5bf99c6](https://github.com/axelhamil/nixos-forge/commit/5bf99c6e117cf4fce88e11e34864a29d93497295))

## [0.2.3](https://github.com/axelhamil/nixos-forge/compare/v0.2.2...v0.2.3) (2026-10-21)


### Features

* **ci:** make every check pass or fail for a reason ([#84](https://github.com/axelhamil/nixos-forge/issues/84)) ([52d1df2](https://github.com/axelhamil/nixos-forge/commit/52d1df2f6a4364f60b03708a879dac704492b057))


### Bug fixes

* **ci:** let the release job take the app's installed permissions ([#86](https://github.com/axelhamil/nixos-forge/issues/86)) ([72c6b79](https://github.com/axelhamil/nixos-forge/commit/72c6b792b08863dbe4dc083a4c123e90d1a208a4))
* **ci:** unblock the release job and the migration test ([#85](https://github.com/axelhamil/nixos-forge/issues/85)) ([4289789](https://github.com/axelhamil/nixos-forge/commit/42897893cf56a8ade41e8051335325500b5c4a3b))

## [0.2.2](https://github.com/axelhamil/nixos-forge/compare/v0.2.1...v0.2.2) (2026-10-21)


### Features

* **ci:** benchmark cpu, memory and disk in the perf job ([#82](https://github.com/axelhamil/nixos-forge/issues/82)) ([1580798](https://github.com/axelhamil/nixos-forge/commit/1580798a229d9f4d339476391551be0bc2623a48))
* **ci:** flag performance regressions against the base on every pr ([#73](https://github.com/axelhamil/nixos-forge/issues/73)) ([77c734f](https://github.com/axelhamil/nixos-forge/commit/77c734f3a5797aa90ca4d0f369e84338cebc0ffa))
* **fido2:** unlock login, sudo and the disk with a security key ([#72](https://github.com/axelhamil/nixos-forge/issues/72)) ([d3d7a94](https://github.com/axelhamil/nixos-forge/commit/d3d7a948cc3d733a5d73f880c609f72739a39afe))


### Bug fixes

* **ci:** fetch the full history before the release moves main ([#67](https://github.com/axelhamil/nixos-forge/issues/67)) ([c19fa91](https://github.com/axelhamil/nixos-forge/commit/c19fa9150da0d44ef0ecf00f44cda770f987b40b))
* **ci:** plan perf in forks and accept overlay packages in apps ([#76](https://github.com/axelhamil/nixos-forge/issues/76)) ([d3ef06a](https://github.com/axelhamil/nixos-forge/commit/d3ef06a40d736c8ead632ca2d0180f8e5da6e47e))
* **init:** catch a deploy key leaked into an untracked fork file ([#75](https://github.com/axelhamil/nixos-forge/issues/75)) ([173e9c6](https://github.com/axelhamil/nixos-forge/commit/173e9c6f00c92a4b11a06bd9ec0f54636e2270d8))
* let forks override policy values without mkForce ([#77](https://github.com/axelhamil/nixos-forge/issues/77)) ([42c2319](https://github.com/axelhamil/nixos-forge/commit/42c2319b4111e99e3060afae07a54c085b353ba1))


### Documentation

* add a how-to for keeping secrets in a fork ([#81](https://github.com/axelhamil/nixos-forge/issues/81)) ([88aaa62](https://github.com/axelhamil/nixos-forge/commit/88aaa62834784dabc72b34e524e62486fa283e1a))
* correct stale claims and make the docs readable for newcomers ([#78](https://github.com/axelhamil/nixos-forge/issues/78)) ([bc54d70](https://github.com/axelhamil/nixos-forge/commit/bc54d7038c1c2d63f844f5d473c10cb70c595ebd))
* give forks docs that stay true after init ([#74](https://github.com/axelhamil/nixos-forge/issues/74)) ([a23ceb9](https://github.com/axelhamil/nixos-forge/commit/a23ceb9a23574a393d548004175dbf0cd7a08633))
* one pitch, newcomer prose and an accurate roadmap ([#83](https://github.com/axelhamil/nixos-forge/issues/83)) ([b241f82](https://github.com/axelhamil/nixos-forge/commit/b241f8212279c5ca8e8ba694bc5b8d57373357ac))
* **roadmap:** plan measured performance and docs, park mode switching ([#70](https://github.com/axelhamil/nixos-forge/issues/70)) ([bf83ab3](https://github.com/axelhamil/nixos-forge/commit/bf83ab3561983e26af104dc29603b516bb55ee96))

## [0.2.1](https://github.com/axelhamil/nixos-forge/compare/v0.2.0...v0.2.1) (2026-10-19)


### Features

* **hosts:** add specialisations variants with steam, gamemode and mangohud apps ([#64](https://github.com/axelhamil/nixos-forge/issues/64)) ([371cc09](https://github.com/axelhamil/nixos-forge/commit/371cc09975f2832111cf5c0e209b12de1cd8adb5))


### Documentation

* **agents:** add the extend-flake skill with recipes for each module kind ([#65](https://github.com/axelhamil/nixos-forge/issues/65)) ([7f44115](https://github.com/axelhamil/nixos-forge/commit/7f44115a5b3205d714872f86a1d75fa39610607e))
* **agents:** state the forge rule and plan lots 37 to 41 ([#66](https://github.com/axelhamil/nixos-forge/issues/66)) ([dfff6ff](https://github.com/axelhamil/nixos-forge/commit/dfff6ffbe5f4c34f9c20beff5c131a8a1e963fe0))
* **readme:** say import-tree skips the files under a _ segment ([#61](https://github.com/axelhamil/nixos-forge/issues/61)) ([db15fad](https://github.com/axelhamil/nixos-forge/commit/db15fadb9b149dfb479fcf2d3220684c03279281))

## [0.2.0](https://github.com/axelhamil/nixos-forge/compare/v0.1.0...v0.2.0) (2026-10-18)


### ⚠ BREAKING CHANGES

* **apps:** a fork migrates its hosts and people:
    - person apps defaults to [ ]: replace shell with zsh starship git neovim fzf zoxide eza bat btop ripgrep fd jq (or the subset wanted), add delta for git diffs, and replace cli with its tools
    - hyprland no longer brings greetd, waybar, fuzzel, kitty, mako, lock: list them (greetd in features, the rest in apps)
    - desktop and laptop no longer bring firmware, audio, bluetooth (add to hardware) nor printing (add printing to features)
    - desktop no longer brings upgrade-notify: add it to the apps of a host that wants the notification

### Features

* **apps:** every app stands alone and the host lists each one ([#42](https://github.com/axelhamil/nixos-forge/issues/42)) ([70d5091](https://github.com/axelhamil/nixos-forge/commit/70d509171e36e19ecc4d46c7264cc0a1bc4036f4))
* **auto-upgrade:** apply only deploy commits ci signed for the host ([#37](https://github.com/axelhamil/nixos-forge/issues/37)) ([76aa566](https://github.com/axelhamil/nixos-forge/commit/76aa56620f77f634843fdd6685769765bc1d3821))
* **auto-upgrade:** prove rollback on a failed health check and migration ([#39](https://github.com/axelhamil/nixos-forge/issues/39)) ([f2177bf](https://github.com/axelhamil/nixos-forge/commit/f2177bf92ff27568bc7cc99ec96cee2c19a2101a))
* **auto-upgrade:** run only what ci validated, roll back a bad boot ([#30](https://github.com/axelhamil/nixos-forge/issues/30)) ([347986f](https://github.com/axelhamil/nixos-forge/commit/347986fe0300a2f394edf59b78ffb72e75bd0669))
* cap the journal, run appimages with nix-ld, load cpu microcode ([#33](https://github.com/axelhamil/nixos-forge/issues/33)) ([ec99ae7](https://github.com/axelhamil/nixos-forge/commit/ec99ae7dddfcfef1c0c9f6cb3e04b864de98bb92))
* **ci:** narvi speaks in reports, update prs and releases ([#43](https://github.com/axelhamil/nixos-forge/issues/43)) ([b149d4c](https://github.com/axelhamil/nixos-forge/commit/b149d4c09f0a1ef2a610388882054b8148b3f1ea))
* **desktop:** print to network printers without installing a driver ([27fb7a6](https://github.com/axelhamil/nixos-forge/commit/27fb7a6aeacaf4365ec6575676d963c2bb7ab9fc))
* **doctor:** check a machine and list what the next boot erases ([6cb9f91](https://github.com/axelhamil/nixos-forge/commit/6cb9f9130bb6fdef323bbba0eb4ab22251d4180c))
* **doctor:** tell what the last upgrade did and notify the desktop after it ([#35](https://github.com/axelhamil/nixos-forge/issues/35)) ([59146b7](https://github.com/axelhamil/nixos-forge/commit/59146b7fc8e9063c1436e8c12d3264a191d006fa))
* **dual-boot:** start windows from the systemd-boot menu with secure boot on ([#44](https://github.com/axelhamil/nixos-forge/issues/44)) ([b6fb897](https://github.com/axelhamil/nixos-forge/commit/b6fb897ce4506885a9142ef1a3eefad80cc76f9a))
* **hibernate:** hibernate and suspend-then-hibernate to an encrypted swap ([bae35e2](https://github.com/axelhamil/nixos-forge/commit/bae35e2c664500e364b557f54ba213fb0ef9e150))
* **hosts:** read the installer's hardware report from a facter field ([259e744](https://github.com/axelhamil/nixos-forge/commit/259e744768d5404806ff9bce7f56ec51e71ed352))
* **init:** make a fresh fork yours with nix run .#init ([393eef3](https://github.com/axelhamil/nixos-forge/commit/393eef3d7edec7425a3277504c0b8052a1099699))
* **installer:** try a full install in a local vm with nix run .#try-install ([#34](https://github.com/axelhamil/nixos-forge/issues/34)) ([3283155](https://github.com/axelhamil/nixos-forge/commit/32831556f303e95bec8d92a856328f8dcc90f3ee))
* **update:** keep forks and machines up to date every week ([#22](https://github.com/axelhamil/nixos-forge/issues/22)) ([2a1f389](https://github.com/axelhamil/nixos-forge/commit/2a1f38905251238d7180a3869ac7a2698fd1fdf2))
* **upstream:** show what the starter changed and prepare the merge on demand ([#31](https://github.com/axelhamil/nixos-forge/issues/31)) ([75d3365](https://github.com/axelhamil/nixos-forge/commit/75d3365449836d16754abc33141647c4ae48a297))
* **wipe-home:** wipe example's home and keep ssh, gpg, gh and checkout ([#32](https://github.com/axelhamil/nixos-forge/issues/32)) ([a9bc51b](https://github.com/axelhamil/nixos-forge/commit/a9bc51bfb2ef6c48ca00678696617510077fa581))


### Bug fixes

* **ci:** drop the close and re-open note from the lock pr body ([#57](https://github.com/axelhamil/nixos-forge/issues/57)) ([0e6ee86](https://github.com/axelhamil/nixos-forge/commit/0e6ee86f3348b96520e74a77ee25991b777b9c48))
* **ci:** run the checks on develop after it merges main ([#56](https://github.com/axelhamil/nixos-forge/issues/56)) ([25e43c3](https://github.com/axelhamil/nixos-forge/commit/25e43c31dcc81ab0fb52e40b317000a96f3a1da3))
* **ci:** show why narvi's anthropic call failed ([#46](https://github.com/axelhamil/nixos-forge/issues/46)) ([12a1c6f](https://github.com/axelhamil/nixos-forge/commit/12a1c6f39cf4089f062add7adee59c22d6dd2c28))
* **ci:** skip the haiku summary when no package changed ([#50](https://github.com/axelhamil/nixos-forge/issues/50)) ([f22e199](https://github.com/axelhamil/nixos-forge/commit/f22e199eb2e98d4cbf14c73950d38871434838a3))
* **ci:** turn off haiku's thinking so narvi's summary fits its budget ([#55](https://github.com/axelhamil/nixos-forge/issues/55)) ([24063c6](https://github.com/axelhamil/nixos-forge/commit/24063c622ce5da78c809ca6b432ee8567607edf7))
* **doctor:** stop reporting links into /run as files lost at boot ([#25](https://github.com/axelhamil/nixos-forge/issues/25)) ([011ad7b](https://github.com/axelhamil/nixos-forge/commit/011ad7bf26ed6ea71358bc295f14bf85f2d6c871))
* **release:** link the fork guide from release prs with an absolute url ([#52](https://github.com/axelhamil/nixos-forge/issues/52)) ([8ae7199](https://github.com/axelhamil/nixos-forge/commit/8ae71996396a1a91acfafbf88edff6328730e4e2))
* **snapshots:** stop the hourly btrbk run from racing the snapshot test ([f20d37f](https://github.com/axelhamil/nixos-forge/commit/f20d37f8f559d122dbd17be834dcfb8be2e6a26f))


### Documentation

* **agents:** say the example hosts rule binds the starter, not forks ([#49](https://github.com/axelhamil/nixos-forge/issues/49)) ([c17e6d9](https://github.com/axelhamil/nixos-forge/commit/c17e6d9ea1ac546491c84dbd893a4f2e4984ac47))
* **hosts:** comment every example host, make the default one a real desktop ([#26](https://github.com/axelhamil/nixos-forge/issues/26)) ([95617f7](https://github.com/axelhamil/nixos-forge/commit/95617f7a26323eee695bbcf9af4049f171826406))
* make the readme and docs fun to read and true to the code ([#47](https://github.com/axelhamil/nixos-forge/issues/47)) ([12dfa62](https://github.com/axelhamil/nixos-forge/commit/12dfa62b3f0e22b2c0a52f41695f9ffcafc6d1ed))
* **readme:** show the weekly commit activity instead of a broken badge ([#58](https://github.com/axelhamil/nixos-forge/issues/58)) ([d1e05b1](https://github.com/axelhamil/nixos-forge/commit/d1e05b1f554b8464048d25d167c9da2fdf0c646a))
* trim the docs to what a fork acts on ([#20](https://github.com/axelhamil/nixos-forge/issues/20)) ([ec19a0c](https://github.com/axelhamil/nixos-forge/commit/ec19a0c3c2661f086e65c571e8dc20ae78632621))

## 0.1.0 (2026-10-16)


### ⚠ BREAKING CHANGES

* declare hosts in a registry that generates their checks

### Features

* **boot:** enforce secure boot with lanzaboote and tpm unlock ([b363a62](https://github.com/axelhamil/nixos-starter/commit/b363a6277c56263ef4aa9350364a9d03b037da03))
* bootstrap a bootable nixos desktop flake ([9ac2350](https://github.com/axelhamil/nixos-starter/commit/9ac2350658250dfcf7360cd108ea9a65950367ee))
* **boot:** unlock the disk with the tpm2 without typing a command ([2bf1a34](https://github.com/axelhamil/nixos-starter/commit/2bf1a340f41bbbb13ba62c2a2ec1345fb6cc4286))
* declare hosts in a registry that generates their checks ([f0c956f](https://github.com/axelhamil/nixos-starter/commit/f0c956fb73088b893a08d144a3580a9d13e9d089))
* **desktop:** log in to a hyprland session from a text greeter ([b25f52a](https://github.com/axelhamil/nixos-starter/commit/b25f52a8a0ba0a3e27cdc89b21e4fda2abc9f399))
* **dev:** add nh, nix-index, rootless docker, libvirt, nix-ld and direnv ([51b7e65](https://github.com/axelhamil/nixos-starter/commit/51b7e65c5709c313b0788a844f2639d1ca3fc83c))
* **disk:** encrypt the disk and wipe / at every boot ([b6f6b5c](https://github.com/axelhamil/nixos-starter/commit/b6f6b5cc91e4f5432ef6a5f9703c3071d3f14a1e))
* **hardware:** add opt-in firmware, audio, bluetooth and gpu modules ([66e427b](https://github.com/axelhamil/nixos-starter/commit/66e427b5c19a3ce7ea94f71fe10677cadbb8308f))
* **home:** add a ready shell, XDG layout and dotfiles linking with stow ([ae38787](https://github.com/axelhamil/nixos-starter/commit/ae387871925860f5eb9e8f77c1f6ca63c8beb732))
* **home:** wipe /home at boot on opt-in, keeping only declared paths ([6116653](https://github.com/axelhamil/nixos-starter/commit/6116653174a290172ee40547e260dc05b61cfa46))
* **hosts:** describe a machine in one file of plain values ([a9c355a](https://github.com/axelhamil/nixos-starter/commit/a9c355a92da59f552ec7089252888349e1fb808d))
* **install:** install a host from a bootable iso that asks a few questions ([570448a](https://github.com/axelhamil/nixos-starter/commit/570448ad737e899740d9751b3e1c8c114c5babf9))
* lint, build and boot every host in CI ([#1](https://github.com/axelhamil/nixos-starter/issues/1)) ([e5c16f9](https://github.com/axelhamil/nixos-starter/commit/e5c16f948b8a271192b9f3ec5fec7bae249fe754))
* **perf:** add desktop, laptop and server profiles with a bench tool ([ad42996](https://github.com/axelhamil/nixos-starter/commit/ad429965ba444ca36108f7b363023f1d2bf7d6e8))
* **pi:** build a raspberry pi 4 or 5 sd card image from the starter ([f103c31](https://github.com/axelhamil/nixos-starter/commit/f103c31fbd53e9c0f4560a9db36f3ab7bf37877e))
* run each check on a runner matching its host system ([77a895c](https://github.com/axelhamil/nixos-starter/commit/77a895c2238b27df4226c3e77a523b02319121da))
* **services:** add resolved, btrbk snapshots, opt-in ssh and secrets ([f0b9721](https://github.com/axelhamil/nixos-starter/commit/f0b9721e2f064e3b2f2486746a1c4d9b3ddef395))


### Bug Fixes

* start the first release at 0.1.0 instead of 1.0.0 ([#2](https://github.com/axelhamil/nixos-starter/issues/2)) ([ed7683d](https://github.com/axelhamil/nixos-starter/commit/ed7683d247c86a57c8a52a000ebeb0b44ac7af46))
