fail() {
  echo "FAIL: $*" >&2
  exit 1
}

export MATRIX EVALUATED GITHUB_STEP_SUMMARY

output() {
  sed -n "s/^$2=//p" "$1"
}

expect() {
  local out=$1 key=$2 filter=$3 value
  value=$(output "$out" "$key")
  jq -e "$filter" <<<"$value" >/dev/null || fail "$out: $key fails '$filter': $value"
}

names() {
  output "$1" "$2" | jq -c 'map(.check)'
}

plan() {
  local event=$1 out=$2 status=0
  GITHUB_STEP_SUMMARY=$PWD/$out.summary
  "$CI_PLAN" "$event" >"$out" 2>"$out.err" || status=$?
  [ "$status" -eq 0 ] || fail "$event: ci-plan exits with $status: $(cat "$out.err")"
}

MATRIX=$PLAN/matrix.json
EVALUATED=$PLAN/evaluated.jsonl

plan pull_request pr.out
[ "$(names pr.out checks)" = '["example-boots","feature-migration","example-pi-sd-image"]' ] || fail "pull_request: wrong checks: $(output pr.out checks)"
expect pr.out skipped '. == [{"check":"example-toplevel","system":"x86_64-linux","retry":false,"reason":"compiles a kernel"},{"check":"docs","system":"x86_64-linux","retry":false,"reason":"cached"}]'
expect pr.out checks '.[0] == {"check":"example-boots","system":"x86_64-linux","runner":"ubuntu-24.04","retry":true,"baseInput":null,"timeout":60}'
expect pr.out checks '.[1].baseInput == "migration-base"'
expect pr.out closures '. == [{"system":"x86_64-linux","runner":"ubuntu-24.04","build":false},{"system":"aarch64-linux","runner":"ubuntu-24.04-arm","build":true}]'
expect pr.out perf ". == $(jq -c . "$PLAN/perfMatrix.json")"
expect pr.out hostChecks ". == $(jq -c . "$PLAN/hostChecks.json")"
expect pr.out descriptions ". == $(jq -c . "$PLAN/descriptions.json")"
grep -qx "3 checks to build." pr.out.summary || fail "pull_request: the summary misses the count: $(cat pr.out.summary)"
grep -qxF -- '- `example-toplevel` (x86_64-linux): compiles a kernel' pr.out.summary || fail "pull_request: the summary misses the kernel skip: $(cat pr.out.summary)"

plan push push.out
[ "$(names push.out checks)" = '["example-toplevel","example-boots","feature-migration","example-pi-sd-image"]' ] || fail "push: wrong checks: $(output push.out checks)"
expect push.out checks '.[0].timeout == 240'
expect push.out skipped '. == [{"check":"docs","system":"x86_64-linux","retry":false,"reason":"cached"}]'

MATRIX=$PLAN/matrix-without-base.json
EVALUATED=$PLAN/cached.jsonl
for event in push pull_request; do
  plan "$event" "empty-$event.out"
  grep -qx 'checks=\[\]' "empty-$event.out" || fail "$event: a fully cached plan does not print checks=[]: $(cat "empty-$event.out")"
  expect "empty-$event.out" skipped 'map(.reason) == ["cached","cached"]'
  expect "empty-$event.out" closures 'all(.build)'
  grep -qx "0 checks to build." "empty-$event.out.summary" || fail "$event: the summary misses the empty count: $(cat "empty-$event.out.summary")"
done

MATRIX=$PLAN/matrix.json
EVALUATED=$PLAN/broken.jsonl
status=0
"$CI_PLAN" push >broken.out 2>broken.err || status=$?
[ "$status" -ne 0 ] || fail "a check that does not evaluate exits with 0: $(cat broken.out)"
grep -qF "x86_64-linux.example-boots does not evaluate: error: undefined variable 'foo'" broken.err || fail "the evaluation error is not named: $(cat broken.err)"
grep -q '^checks=' broken.out && fail "a plan was printed despite the evaluation error: $(cat broken.out)"

EVALUATED=$PLAN/missing.jsonl
status=0
"$CI_PLAN" push >missing.out 2>missing.err || status=$?
[ "$status" -ne 0 ] || fail "a check missing from the evaluation exits with 0: $(cat missing.out)"
grep -qF "x86_64-linux.example-boots is missing from the evaluation" missing.err || fail "the missing check is not named: $(cat missing.err)"

echo "ci-plan: all scenarios passed"
