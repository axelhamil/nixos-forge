base_rev=$(git rev-parse "$1")
base_flake="git+file://$PWD?rev=$base_rev"

has_host() {
  [ "$(nix eval --json "$1#nixosConfigurations" --apply "hosts: hosts ? \"$2\"")" = true ]
}

echo "### Closure diff"

system=$(nix eval --impure --raw --expr builtins.currentSystem)

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
