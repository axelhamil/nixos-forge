base_rev=$(git rev-parse "$1")
base_flake="git+file://$PWD?rev=$base_rev"

has_host() {
  [ "$(nix eval --json "$1#nixosConfigurations" --apply "hosts: hosts ? \"$2\"")" = true ]
}

system=$(nix eval --impure --raw --expr builtins.currentSystem)

echo "### Closure diff ($system)"

if [ "${2:-}" = --no-build ]; then
  echo
  echo "Not built: it needs a kernel that is not in the binary cache yet, and PRs never compile one. The push to \`develop\` after the merge builds and caches it."
  exit 0
fi

for host in $(nix eval --raw ".#ci.hosts.$system" --apply 'builtins.concatStringsSep " "'); do
  toplevel="nixosConfigurations.$host.config.system.build.toplevel"
  after=$(nix build --no-link --print-out-paths ".#$toplevel")

  echo
  echo "#### $host"
  echo '```'

  if has_host "$base_flake" "$host"; then
    before=$(nix build --no-link --print-out-paths "$base_flake#$toplevel")
    nvd diff "$before" "$after"
  else
    echo "new host"
  fi

  echo '```'
done
