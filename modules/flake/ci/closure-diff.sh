base_rev=$1
base_flake="git+file://$PWD?rev=$base_rev"

has_host() {
  [ "$(nix eval --json "$1#nixosConfigurations" --apply "hosts: hosts ? \"$2\"")" = true ]
}

echo "### Closure diff"

for host in $(nix eval --raw .#nixosConfigurations --apply 'hosts: builtins.concatStringsSep " " (builtins.attrNames hosts)'); do
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
