fail() {
  printf 'init-test: %s\n' "$*" >&2
  exit 1
}

fresh_fork() {
  local dir=$1
  cp -r "$SOURCE" "$dir"
  chmod -R u+w "$dir"
  git -C "$dir" init -q
  git -C "$dir" add -A
  git -C "$dir" -c user.name=test -c user.email=test@example.org commit -q -m fork
}

has_line() {
  grep -qxE -- "$1" "$host_file" || fail "$host_file has no line matching: $1"
}

refuses() {
  local expected=$1 output
  shift
  if output=$(init "$@" 2>&1 </dev/null); then
    fail "init $* succeeded, expected: $expected"
  fi
  grep -qF -- "$expected" <<<"$output" || fail "init $* failed with: $output"
}

export NIX_STATE_DIR=$TMPDIR/nix-state
export NIX_STORE_DIR=$TMPDIR/nix-store

fork=$TMPDIR/fork
fresh_fork "$fork"
cd "$fork"

init --yes \
  --host my-box \
  --profile laptop \
  --login bob \
  --full-name "Bob \"the\" Builder \${x}" \
  --email bob@example.net \
  --time-zone Europe/Paris \
  --keyboard fr \
  --locale fr_FR.UTF-8 \
  --features disk,docker \
  --hardware 'nvidia framework-13-7040-amd' \
  --keep-pi

host_file=modules/hosts/my-box/default.nix
[[ -f $host_file ]] || fail "$host_file was not written"

"$NIX_INSTANTIATE" --parse "$host_file" >/dev/null || fail "$host_file does not parse"
nixfmt --check "$host_file" || fail "$host_file is not formatted"

has_line '  configurations\.nixos\.my-box = \{'
has_line '    system = "x86_64-linux";.*'
has_line '    profile = "laptop";.*'
has_line '    flake = "/home/bob/nixos";'
has_line '    timeZone = "Europe/Paris";.*'
has_line '    keyboard = "fr";.*'
has_line '    locale = "fr_FR\.UTF-8";.*'
has_line '    users\.bob = \{'
has_line '      fullName = "Bob \\"the\\" Builder \\\$\{x\}";.*'
has_line '      email = "bob@example\.net";.*'
has_line '      "disk" # .*'
has_line '      "docker" # .*'
has_line '      # "hyprland" # .*'
has_line '      # "secure-boot" # .*'
has_line '      # "libvirt" # .*'
has_line '      "nvidia" # .*'
has_line '      "framework-13-7040-amd"'
has_line '      # "common-pc-ssd" # .*'
has_line '      "firefox" # .*'
if grep -q alice "$host_file"; then
  fail "$host_file still names the example user"
fi

[[ ! -e modules/hosts/example && ! -e modules/hosts/example-server ]] || fail "example hosts left on disk"
[[ -f modules/hosts/example-pi/default.nix ]] || fail "--keep-pi removed example-pi"

changes=$(git status --porcelain --no-renames | sort)
expected=$(printf '%s\n' \
  "A  $host_file" \
  "D  modules/hosts/example-server/default.nix" \
  "D  modules/hosts/example/default.nix" | sort)
[[ $changes == "$expected" ]] || fail "unexpected changes: $changes"

refuses "is missing" --yes --host other --login bob --full-name Bob --email ""

second=$TMPDIR/second
fresh_fork "$second"
cd "$second"

refuses "already exists" --yes --host example-server --login bob --full-name Bob --email "" --keep-server
refuses "1 to 63" --yes --host -bad --login bob --full-name Bob --email ""
refuses "--login is required" --yes --host box --full-name Bob --email ""
refuses "--email is required" --yes --host box --login bob --full-name Bob
refuses "unknown feature" --yes --host box --login bob --full-name Bob --email "" --features disk,nope
[[ -z $(git status --porcelain) ]] || fail "a refused run changed the checkout"

init --yes --host pi-box --system aarch64-linux --login bob --full-name Bob --email ""
host_file=modules/hosts/pi-box/default.nix
"$NIX_INSTANTIATE" --parse "$host_file" >/dev/null || fail "$host_file does not parse"
nixfmt --check "$host_file" || fail "$host_file is not formatted"
has_line '    system = "aarch64-linux";.*'
has_line '    profile = "pi";.*'
has_line '      "ssh" # .*'
has_line '      # "disk" # .*'
has_line '      "raspberry-pi-5" # .*'
has_line '      # "firefox" # .*'
has_line '      # email = "alice@example\.org";.*'
[[ ! -e modules/hosts/example-pi ]] || fail "example-pi left without --keep-pi"

third=$TMPDIR/third
fresh_fork "$third"
cd "$third"

init --yes --host rec --login in --full-name In --email ""
host_file=modules/hosts/rec/default.nix
"$NIX_INSTANTIATE" --parse "$host_file" >/dev/null || fail "$host_file does not parse"
has_line '  configurations\.nixos\."rec" = \{'
has_line '    users\."in" = \{'

echo "init-test: all assertions passed"
