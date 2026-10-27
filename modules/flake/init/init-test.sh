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

parses_and_is_formatted() {
  local file
  for file in "$@"; do
    "$NIX_INSTANTIATE" --parse "$file" >/dev/null || fail "$file does not parse"
    nixfmt --check "$file" || fail "$file is not formatted"
  done
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

docs_pass() {
  local output
  output=$(forge check-docs "$PWD" 2>&1) || fail "the docs of the fork in $PWD fail: $output"
}

export NIX_STATE_DIR=$TMPDIR/nix-state
export NIX_STORE_DIR=$TMPDIR/nix-store
export XDG_RUNTIME_DIR=$TMPDIR/runtime
mkdir -m 700 "$XDG_RUNTIME_DIR"

fresh_fork "$TMPDIR/first"
cd "$TMPDIR/first"
init --yes \
  --host my-box \
  --profile laptop \
  --login bob \
  --full-name "Bob \"the\" Builder \${x}" \
  --email bob@example.net \
  --features disk,docker \
  --hardware 'nvidia framework-13-7040-amd' \
  --apps firefox,vlc \
  --keep-pi >"$TMPDIR/init.out"
parses_and_is_formatted modules/hosts/my-box/default.nix modules/users/bob/default.nix modules/hosts/example-pi/default.nix
key_command=$(grep -E '^  gh secret set DEPLOY_SIGNING_KEY --env deploy < [^ ]+ && shred -u [^ ]+$' "$TMPDIR/init.out") || fail "init does not print the command storing the private key: $(cat "$TMPDIR/init.out")"
key_file=${key_command##* }
public_key=$(ssh-keygen -y -f "$key_file" | cut -d' ' -f1,2) || fail "init printed no readable key file: $key_file"
grep -qxF "      \"$public_key deploy\"" modules/hosts/my-box/default.nix || fail "the signers of my-box do not hold the public key of $key_file"
fork_evaluates '["example-pi","my-box"]'

fresh_fork "$TMPDIR/second"
cd "$TMPDIR/second"
init --yes --host pi-box --system aarch64-linux --login bob --full-name Bob --email "" >/dev/null
parses_and_is_formatted modules/hosts/pi-box/default.nix modules/users/bob/default.nix
fork_evaluates '["pi-box"]'

fresh_fork "$TMPDIR/third"
cd "$TMPDIR/third"
init --yes --host rec --login in --full-name In --email "" >/dev/null
parses_and_is_formatted modules/hosts/rec/default.nix modules/users/in/default.nix
fork_evaluates '["rec"]'
perf_systems=$("$NIX_INSTANTIATE" --eval --strict --json --expr "import $FORK_PERF_SYSTEMS $PWD") || fail "ci.perfMatrix of the fork in $PWD does not evaluate"
[[ $perf_systems == '["x86_64-linux"]' ]] || fail "the fork in $PWD, whose host is not on the starter, gets the perf job on $perf_systems, expected [\"x86_64-linux\"]"
docs_pass

fresh_fork "$TMPDIR/fourth"
cd "$TMPDIR/fourth"
init --yes --host keeps-alice --login alice --full-name "Alice Fork" --email "" --keep-server >/dev/null
parses_and_is_formatted modules/hosts/example-server/default.nix
fork_evaluates '["example-server","keeps-alice"]'

fresh_fork "$TMPDIR/fifth"
cd "$TMPDIR/fifth"
git remote add origin git@github.com:bob/my-nixos.git
init --yes --host box --login bob --full-name Bob --email "" --keep-server --keep-pi >/dev/null
parses_and_is_formatted modules/hosts/example-server/default.nix modules/hosts/example-pi/default.nix
fork_evaluates '["box","example-pi","example-server"]'
docs_pass

fresh_fork "$TMPDIR/sixth"
cd "$TMPDIR/sixth"
init --yes --host typo --profile server --login dan --full-name Dan --email "" --apps nope-not-a-package >/dev/null
fork_fails 'configurations.nixos.typo.apps: unknown name "nope-not-a-package"'
git reset -q --hard
git clean -q -fd
init --yes --host typo --profile server --login dan --full-name Dan --email "" >/dev/null
person_file=modules/users/dan/default.nix
fork_evaluates '["typo"]'

sed -i 's/^    fullName = .*/    fullName = 3;/' "$person_file"
fork_fails "is not of type"
git checkout -q -- "$person_file"

sed -i 's/^    apps = \[$/    apps = [ "nope-not-a-package"/' "$person_file"
fork_fails 'users.dan.apps (modules/users/dan): unknown name "nope-not-a-package"'
git checkout -q -- "$person_file"

sed -i 's/^    # module = .*/    module.programs.no-such-program.enable = true;/' "$person_file"
fork_fails "no-such-program' does not exist"
git checkout -q -- "$person_file"
fork_evaluates '["typo"]'

echo "init-test: all assertions passed"
