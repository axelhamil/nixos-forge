fail() {
  echo "FAIL: $*" >&2
  exit 1
}

expect_excerpt() {
  local log=$1 pattern=$2 excerpt
  excerpt=$(awk -f "$EXCERPT" "$LOGS/$log")
  grep -q -- "$pattern" <<<"$excerpt" || fail "$log: excerpt misses '$pattern': $excerpt"
  [ "$(wc -l <<<"$excerpt")" -le 20 ] || fail "$log: excerpt has more than 20 lines"
  if grep -q $'\033' <<<"$excerpt"; then fail "$log: excerpt keeps terminal escapes"; fi
  [ "$(sort <<<"$excerpt" | uniq -d)" = "" ] || fail "$log: excerpt repeats a line"
}

expect_excerpt assertion.log "^AssertionError: doctor misses a file written to /$"
expect_excerpt assertion.log "^Traceback (most recent call last):$"
expect_excerpt eval-error.log "^error: undefined variable 'foo'$"
expect_excerpt timeout.log "^RequestedAssertionFailed: action timed out after 120.73 seconds"

seq -f "machine # line %g of a healthy boot" 5000 >quiet.log
[ "$(awk -f "$EXCERPT" quiet.log | wc -l)" -eq 0 ] || fail "a log without failure yields an excerpt"

export NIX_CALLS

NIX_CALLS=$PWD/flaky-calls
if ! "$FLAKY_CI_BUILD" x86_64-linux example-boots results --retry >flaky.out; then
  fail "a vm test passing on its second attempt exits non-zero"
fi
result=results/example-boots.json
jq -e '.check == "example-boots" and .system == "x86_64-linux"' "$result" >/dev/null || fail "flaky: wrong check or system: $(cat "$result")"
jq -e '.outcome == "unstable" and .attempts == 2' "$result" >/dev/null || fail "flaky: not unstable after 2 attempts: $(cat "$result")"
jq -e '.excerpt | any(test("^RequestedAssertionFailed: action timed out"))' "$result" >/dev/null || fail "flaky: excerpt misses the first failure: $(cat "$result")"
[ "$(wc -l <"$NIX_CALLS")" -eq 2 ] || fail "flaky: nix called $(wc -l <"$NIX_CALLS") times"
grep -qx "build --no-link --print-build-logs .#checks.x86_64-linux.example-boots" "$NIX_CALLS" || fail "flaky: unexpected nix call: $(cat "$NIX_CALLS")"
grep -q "action timed out" flaky.out || fail "flaky: the full log is not on stdout"

NIX_CALLS=$PWD/broken-calls
status=0
"$BROKEN_CI_BUILD" x86_64-linux example-toplevel results >broken.out || status=$?
[ "$status" -eq 1 ] || fail "a failing check exits with $status instead of 1"
result=results/example-toplevel.json
jq -e '.outcome == "failed" and .attempts == 1' "$result" >/dev/null || fail "broken: not failed after 1 attempt: $(cat "$result")"
jq -e '.excerpt == ["error: undefined variable '"'"'foo'"'"'"]' "$result" >/dev/null || fail "broken: wrong excerpt: $(cat "$result")"
[ "$(wc -l <"$NIX_CALLS")" -eq 1 ] || fail "broken: a check without --retry was built $(wc -l <"$NIX_CALLS") times"

NIX_CALLS=$PWD/broken-vm-calls
status=0
"$BROKEN_CI_BUILD" x86_64-linux example-disk results --retry >broken-vm.out || status=$?
[ "$status" -eq 1 ] || fail "a vm test failing twice exits with $status instead of 1"
jq -e '.outcome == "failed" and .attempts == 2' results/example-disk.json >/dev/null || fail "broken vm: not failed after 2 attempts: $(cat results/example-disk.json)"

echo "ci-report: all assertions passed"
