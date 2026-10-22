fail() {
  echo "FAIL: $*" >&2
  exit 1
}

export CALLS CACHIX_AUTH_TOKEN CACHIX_FAIL

held_out=/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-linux-6.18
held_dev=/nix/store/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb-linux-6.18-dev
missing=/nix/store/cccccccccccccccccccccccccccccccc-linux-6.18-modules

run() {
  local out=$1 status=0
  CALLS=$PWD/$out.calls
  : >"$CALLS"
  shift
  "$CI_PIN_KERNELS" "$@" >"$out" 2>"$out.err" || status=$?
  echo "$status"
}

CACHIX_AUTH_TOKEN=token
CACHIX_FAIL=

status=$(run held.out forge)
[ "$status" -eq 0 ] || fail "pins that succeed exit with $status: $(cat held.out.err)"
grep -qxF "cachix pin forge kernel-example-out $held_out --keep-revisions 3" held.out.calls || fail "the out kernel is not pinned: $(cat held.out.calls)"
grep -qxF "cachix pin forge kernel-example-dev $held_dev --keep-revisions 3" held.out.calls || fail "the dev kernel is not pinned: $(cat held.out.calls)"
[ "$(grep -c '^cachix' held.out.calls)" -eq 2 ] || fail "wrong number of pins: $(cat held.out.calls)"
grep -qxF "pinned kernel-example-out $held_out" held.out || fail "the out pin is not reported: $(cat held.out)"
grep -qxF "pinned kernel-example-dev $held_dev" held.out || fail "the dev pin is not reported: $(cat held.out)"
grep -qxF "not in forge, not pinned: kernel-example-modules $missing" held.out || fail "the missing path is not reported: $(cat held.out)"

CACHIX_FAIL=kernel-example-out
status=$(run failing.out forge)
[ "$status" -ne 0 ] || fail "a failing pin exits with 0: $(cat failing.out)"
grep -qxF "cachix pin forge kernel-example-dev $held_dev --keep-revisions 3" failing.out.calls || fail "a failing pin stops the next ones: $(cat failing.out.calls)"
grep -qF "failed to pin kernel-example-out $held_out" failing.out.err || fail "the failing pin is not named: $(cat failing.out.err)"
CACHIX_FAIL=

CACHIX_AUTH_TOKEN=
status=$(run no-token.out forge)
[ "$status" -ne 0 ] || fail "an empty token exits with 0"
[ ! -s no-token.out.calls ] || fail "an empty token still calls: $(cat no-token.out.calls)"
grep -qF "CACHIX_AUTH_TOKEN is empty" no-token.out.err || fail "an empty token is not explained: $(cat no-token.out.err)"

CACHIX_AUTH_TOKEN=token
status=$(run no-args.out)
[ "$status" -eq 2 ] || fail "no argument exits with $status instead of 2"
grep -qF "usage: ci-pin-kernels <cache-name>" no-args.out.err || fail "no argument prints no usage: $(cat no-args.out.err)"
[ ! -s no-args.out.calls ] || fail "no argument still calls: $(cat no-args.out.calls)"

echo "ci-pin-kernels: all scenarios passed"
