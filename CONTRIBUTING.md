# Contributing

Thanks for helping improve the starter. Architecture, conventions and pitfalls are in [AGENTS.md](AGENTS.md); read it before a non-trivial change.

## Setup

```sh
git clone https://github.com/axelhamil/nixos-starter.git
cd nixos-starter
nix develop
```

`nix develop` installs a pre-push hook (treefmt, `nix flake check --no-build`, actionlint) that runs in seconds. Requires Nix with flakes, and KVM to run the boot tests.

## Commands

| Command | What it does |
|---|---|
| `nix fmt` | Format and lint (nixfmt, statix, deadnix) |
| `nix flake check -L` | Lint, build every host, boot every host in a VM test |
| `nix build .#nixosConfigurations.example.config.system.build.vm` | Build the example VM |
| `./result/bin/run-example-vm` | Run it (user `alice`, password `test`) |

## Commit messages

[Conventional Commits](https://www.conventionalcommits.org/) drive releases through release-please. While in `0.x`, `feat` and breaking changes bump the minor, `fix` the patch, and `chore`, `ci`, `docs` and `refactor` do not release.

```text
feat: add a server profile
fix: keep the boot test working without home-manager users
```

## Pull requests

1. Branch from `develop` (`feat/`, `fix/`, `refactor/`, `chore/`, `docs/` plus a slug).
2. Run `nix fmt` and `nix flake check -L`.
3. Open the PR against `develop`. CI builds and boots every host and posts the package diff.
