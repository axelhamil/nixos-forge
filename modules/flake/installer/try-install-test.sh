fail() {
  echo "FAIL: $*" >&2
  exit 1
}

expect_exit() {
  local code=$1 pattern=$2
  shift 2
  local output status=0
  output=$(try-install "$@" 2>&1) || status=$?
  [[ $status == "$code" ]] || fail "try-install $* exited $status, expected $code: $output"
  grep -q -- "$pattern" <<<"$output" || fail "try-install $* did not say '$pattern': $output"
}

export HOME=$PWD/home XDG_STATE_HOME=$PWD/state

expect_exit 0 "Usage: nix run .#try-install" --help
expect_exit 1 "never touches a real disk" example /dev/sda
expect_exit 1 "never touches a real disk" /dev/nvme0n1
expect_exit 1 "never touches a real disk" example --disk-size /dev/sdb
expect_exit 1 "no installer for nowhere, hosts with one: example" nowhere
expect_exit 1 "one host only" example example-server
expect_exit 1 "unknown option --nope" example --nope
expect_exit 1 "--disk-size takes a size" example --disk-size lots
expect_exit 1 "name a host" --disk-size 8G

[[ ! -e $XDG_STATE_HOME ]] || fail "a refused run left state in $XDG_STATE_HOME"
