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

person_has_line() {
  grep -qxE -- "$1" "$person_file" || fail "$person_file has no line matching: $1"
}

toplevels() {
  "$NIX_INSTANTIATE" --eval --strict --json --expr "
    let toplevels = import $FORK_TOPLEVELS $PWD;
    in builtins.deepSeq toplevels (builtins.attrNames toplevels)
  "
}

fork_evaluates() {
  local hosts
  hosts=$(toplevels) || fail "a host of the fork in $PWD does not evaluate"
  [[ $hosts == "$1" ]] || fail "the fork in $PWD has the hosts $hosts, expected $1"
}

fork_fails() {
  local expected=$1 output
  if output=$(toplevels 2>&1); then
    fail "the fork in $PWD evaluated, expected: $expected"
  fi
  grep -qF -- "$expected" <<<"$output" || fail "the fork in $PWD failed with: $output"
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
  --apps firefox,vlc \
  --keep-pi

host_file=modules/hosts/my-box/default.nix
[[ -f $host_file ]] || fail "$host_file was not written"

"$NIX_INSTANTIATE" --parse "$host_file" >/dev/null || fail "$host_file does not parse"
nixfmt --check "$host_file" || fail "$host_file is not formatted"

has_line '  configurations\.nixos\.my-box = \{'
has_line '    system = "x86_64-linux";.*'
has_line '    profile = "laptop";.*'
has_line '    flake = "/home/bob/nixos";'
has_line '    # mirrors = \[ "https://codeberg.org/bob/nixos.git" \]; # .*'
has_line '    timeZone = "Europe/Paris";.*'
has_line '    keyboard = "fr";.*'
has_line '    locale = "fr_FR\.UTF-8";.*'
has_line '    users\.bob = \{'
has_line '      "disk" # .*'
has_line '      "docker" # .*'
has_line '      # "secure-boot" # .*'
has_line '      # "libvirt" # .*'
has_line '      "nvidia" # .*'
has_line '      "framework-13-7040-amd"'
has_line '      # "common-pc-ssd" # .*'
has_line '      "firefox" # .*'
has_line '      "vlc"'
has_line '      # "hyprland" # .*'
has_line '      # "direnv" # .*'
if grep -q alice "$host_file"; then
  fail "$host_file still names the example user"
fi

person_file=modules/users/bob/default.nix
[[ -f $person_file ]] || fail "$person_file was not written"
"$NIX_INSTANTIATE" --parse "$person_file" >/dev/null || fail "$person_file does not parse"
nixfmt --check "$person_file" || fail "$person_file is not formatted"
person_has_line '  users\.bob = \{'
person_has_line '    fullName = "Bob \\"the\\" Builder \\\$\{x\}";.*'
person_has_line '    email = "bob@example\.net";.*'
if grep -q alice "$person_file"; then
  fail "$person_file still names the example person"
fi
[[ ! -e modules/users/alice ]] || fail "the example person left on disk"
grep -qxF '    users.bob = {' modules/hosts/example-pi/default.nix || fail "the kept example-pi does not give bob his account"

[[ ! -e modules/hosts/example && ! -e modules/hosts/example-server ]] || fail "example hosts left on disk"
[[ -f modules/hosts/example-pi/default.nix ]] || fail "--keep-pi removed example-pi"

changes=$(git status --porcelain --no-renames | sort)
expected=$(printf '%s\n' \
  "A  $host_file" \
  "A  $person_file" \
  "D  modules/hosts/example-server/default.nix" \
  "D  modules/hosts/example/default.nix" \
  "D  modules/users/alice/default.nix" \
  "M  modules/hosts/example-pi/default.nix" | sort)
[[ $changes == "$expected" ]] || fail "unexpected changes: $changes"
fork_evaluates '["example-pi","my-box"]'

refuses "is missing" --yes --host other --login bob --full-name Bob --email ""

second=$TMPDIR/second
fresh_fork "$second"
cd "$second"

refuses "already exists" --yes --host example-server --login bob --full-name Bob --email "" --keep-server
refuses "1 to 63" --yes --host -bad --login bob --full-name Bob --email ""
refuses "--login is required" --yes --host box --full-name Bob --email ""
refuses "--email is required" --yes --host box --login bob --full-name Bob
refuses "unknown feature" --yes --host box --login bob --full-name Bob --email "" --features disk,nope
mkdir -p modules/users/carol
refuses "modules/users/carol already exists" --yes --host box --login carol --full-name Carol --email ""
rmdir modules/users/carol
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
has_line '      # "hyprland" # .*'
person_file=modules/users/bob/default.nix
person_has_line '    # email = "bob@example\.org";.*'
[[ ! -e modules/hosts/example-pi ]] || fail "example-pi left without --keep-pi"
fork_evaluates '["pi-box"]'

third=$TMPDIR/third
fresh_fork "$third"
cd "$third"

init --yes --host rec --login in --full-name In --email ""
host_file=modules/hosts/rec/default.nix
"$NIX_INSTANTIATE" --parse "$host_file" >/dev/null || fail "$host_file does not parse"
has_line '  configurations\.nixos\."rec" = \{'
has_line '    users\."in" = \{'
has_line '      "hyprland" # .*'
has_line '      "direnv" # .*'
has_line '      "nix-ld" # .*'
person_file=modules/users/in/default.nix
person_has_line '  users\."in" = \{'
person_has_line '.* an account with .users\."in".*'
fork_evaluates '["rec"]'

fourth=$TMPDIR/fourth
fresh_fork "$fourth"
cd "$fourth"

init --yes --host keeps-alice --login alice --full-name "Alice Fork" --email "" --keep-server
person_file=modules/users/alice/default.nix
person_has_line '    fullName = "Alice Fork";.*'
grep -qxF '    users.alice = {' modules/hosts/example-server/default.nix || fail "example-server lost the account of alice"
host_file=modules/hosts/example-server/default.nix
has_line '      # "auto-upgrade" # .*'
has_line '    # flake = "github:alice/nixos";'
fork_evaluates '["example-server","keeps-alice"]'

kept_flake() {
  local dir=$1 origin=$2
  fresh_fork "$dir"
  cd "$dir"
  git remote add origin "$origin"
  init --yes --host box --login bob --full-name Bob --email "" --keep-server --keep-pi
  host_file=modules/hosts/example-server/default.nix
}

kept_flake "$TMPDIR/sixth" git@github.com:bob/my-nixos.git
has_line '      "auto-upgrade" # .*'
has_line '    flake = "github:bob/my-nixos";'
host_file=modules/hosts/example-pi/default.nix
has_line '    # flake = "github:bob/my-nixos";'
fork_evaluates '["box","example-pi","example-server"]'

kept_flake "$TMPDIR/seventh" https://git.example.org/bob/nixos.git
has_line '      "auto-upgrade" # .*'
has_line '    flake = "git\+https://git\.example\.org/bob/nixos\.git";'

kept_flake "$TMPDIR/eighth" https://github.com/axelhamil/nixos-starter.git
has_line '      # "auto-upgrade" # .*'
has_line '    # flake = "github:alice/nixos";'

fifth=$TMPDIR/fifth
fresh_fork "$fifth"
cd "$fifth"

init --yes --host typo --profile server --login dan --full-name Dan --email "" --apps nope-not-a-package
fork_fails 'configurations.nixos.typo.apps: unknown name "nope-not-a-package"'
git reset -q --hard
git clean -q -fd
init --yes --host typo --profile server --login dan --full-name Dan --email ""
person_file=modules/users/dan/default.nix
fork_evaluates '["typo"]'

sed -i 's/^    fullName = .*/    fullName = 3;/' "$person_file"
fork_fails "is not of type"
git checkout -q -- "$person_file"

sed -i 's/^    # apps = .*/    apps = [ "shell" "nope-not-a-package" ];/' "$person_file"
fork_fails 'users.dan.apps (modules/users/dan): unknown name "nope-not-a-package"'
git checkout -q -- "$person_file"

sed -i 's/^    # module = .*/    module.programs.no-such-program.enable = true;/' "$person_file"
fork_fails "no-such-program' does not exist"
git checkout -q -- "$person_file"
fork_evaluates '["typo"]'

echo "init-test: all assertions passed"
