fail() {
  echo "FAIL: $*" >&2
  exit 1
}

unset ANTHROPIC_API_KEY ANTHROPIC_FEDERATION_RULE_ID ANTHROPIC_ORGANIZATION_ID ANTHROPIC_SERVICE_ACCOUNT_ID ANTHROPIC_WORKSPACE_ID
unset ACTIONS_ID_TOKEN_REQUEST_URL ACTIONS_ID_TOKEN_REQUEST_TOKEN GITHUB_RUN_ID GITHUB_SHA RUNNER_TEMP NARVI_CACHE_DIR

jq -e 'to_entries | all(.value | type == "array" and length >= 5 and length <= 10 and all(type == "string" and . != ""))' "$MESSAGES" >/dev/null ||
  fail "every moment of messages.json needs 5 to 10 lines"
if jq -r '.[][]' "$MESSAGES" | LC_ALL=C grep -n '[^ -~]\|[*_`]\| - '; then
  fail "a line of messages.json has a character outside plain ASCII, a dash or Markdown"
fi
for moment in report-green report-red lock-pr upstream-pr release; do
  [ -n "$(NARVI_MESSAGES=$MESSAGES "$NARVI" say "$moment")" ] || fail "messages.json says nothing for $moment"
done

cat >messages.json <<'EOF'
{
  "report-green": ["one", "two", "three", "four", "five"],
  "report-red": [],
  "broken": "not a list"
}
EOF
export NARVI_MESSAGES=$PWD/messages.json

first=$("$NARVI" say report-green 1234)
[ "$("$NARVI" say report-green 1234)" = "$first" ] || fail "the same seed picks another line"
jq -e --arg line "$first" '."report-green" | index($line)' messages.json >/dev/null || fail "say printed '$first', not a line of the list"
[ "$(GITHUB_RUN_ID=1234 "$NARVI" say report-green)" = "$first" ] || fail "GITHUB_RUN_ID is not the default seed"
[ "$(GITHUB_SHA=1234 "$NARVI" say report-green)" = "$first" ] || fail "GITHUB_SHA is not the seed without GITHUB_RUN_ID"
picked=$(for seed in $(seq 1 40); do "$NARVI" say report-green "$seed"; done | sort -u | wc -l)
[ "$picked" -ge 3 ] || fail "40 seeds picked only $picked distinct lines"

for moment in report-red broken unknown; do
  out=$("$NARVI" say "$moment" 7) || fail "say $moment exits non-zero"
  [ -z "$out" ] || fail "say $moment printed '$out' for an empty, broken or missing list"
done
out=$(NARVI_MESSAGES=$PWD/missing.json "$NARVI" say report-green 2>missing.err) || fail "a missing messages.json exits non-zero"
[ -z "$out" ] || fail "a missing messages.json printed '$out'"
grep -q "cannot read" missing.err || fail "a missing messages.json is not warned about: $(cat missing.err)"

status=0
"$NARVI" summarize nonsense diff.md 2>/dev/null || status=$?
[ "$status" -eq 2 ] || fail "an unknown kind exits with $status instead of 2"

printf '### Closure diff (x86_64-linux)\n\n#### example\n```\n[U.] linux 6.12 -> 6.18\n[U.] firefox 140 -> 141\n```\n' >diff.md

echo ok >mode
python3 "$STUB" "$PWD/requests.jsonl" "$PWD/port" "$PWD/mode" &
stub=$!
trap 'kill "$stub" 2>/dev/null || true' EXIT
for _ in $(seq 50); do
  [ -s port ] && break
  sleep 0.1
done
[ -s port ] || fail "the stub server did not start"
ANTHROPIC_BASE_URL=http://127.0.0.1:$(cat port)
export ANTHROPIC_BASE_URL
summary="The kernel moves to 6.18, Firefox jumps a major version."
secrets="sk-ant-oat01-stub\|jwt-from-github\|gh-request-token\|sk-ant-api-test"

run() {
  local name=$1
  shift
  : >requests.jsonl
  "$@" >"$name.out" 2>"$name.err" || fail "$name: exits non-zero: $(cat "$name.err")"
  if grep -q "$secrets" "$name.out" "$name.err"; then fail "$name: a token shows in the output"; fi
}

request() {
  jq -c --arg path "$1" 'select(.path | startswith($path))' requests.jsonl
}

oidc() {
  ACTIONS_ID_TOKEN_REQUEST_URL="$ANTHROPIC_BASE_URL/oidc?api-version=2.0" \
    ACTIONS_ID_TOKEN_REQUEST_TOKEN=gh-request-token \
    ANTHROPIC_FEDERATION_RULE_ID=fdrl_test \
    ANTHROPIC_ORGANIZATION_ID=00000000-0000-0000-0000-000000000000 \
    ANTHROPIC_SERVICE_ACCOUNT_ID=svac_test \
    "$@"
}

run oidc oidc env ANTHROPIC_WORKSPACE_ID=wrkspc_test ANTHROPIC_API_KEY=sk-ant-api-test "$NARVI" summarize packages diff.md
[ "$(cat oidc.out)" = "$summary" ] || fail "oidc: the summary is '$(cat oidc.out)', expected '$summary'"
request /oidc | jq -e '.method == "GET" and (.path | endswith("&audience=https://api.anthropic.com"))' >/dev/null ||
  fail "oidc: GitHub's token was not asked for the Anthropic audience: $(cat requests.jsonl)"
request /v1/oauth/token | jq -e '.body == {
    grant_type: "urn:ietf:params:oauth:grant-type:jwt-bearer",
    assertion: "jwt-from-github",
    federation_rule_id: "fdrl_test",
    organization_id: "00000000-0000-0000-0000-000000000000",
    service_account_id: "svac_test",
    workspace_id: "wrkspc_test"
  }' >/dev/null || fail "oidc: wrong exchange body: $(request /v1/oauth/token)"
request /v1/messages | jq -e '.authorization == "Bearer sk-ant-oat01-stub" and ."x-api-key" == "" and ."anthropic-version" == "2023-06-01"' >/dev/null ||
  fail "oidc: the Messages call does not carry the federated token alone: $(request /v1/messages)"
request /v1/messages | jq -e '.body.model == "claude-haiku-5-5" and .body.max_tokens <= 500 and (.body.system | test("never use a dash")) and (.body.messages[0].content | test("linux 6.12 -> 6.18"))' >/dev/null ||
  fail "oidc: wrong Messages body: $(request /v1/messages)"
[ "$(jq -s 'map(.path | sub("[?].*"; "")) | join(" ")' requests.jsonl)" = '"/oidc /v1/oauth/token /v1/messages"' ] ||
  fail "oidc: the calls are not GitHub, the exchange, then Messages: $(cat requests.jsonl)"

run no-workspace oidc "$NARVI" summarize packages diff.md
request /v1/oauth/token | jq -e '.body | has("workspace_id") | not' >/dev/null || fail "no-workspace: an empty workspace_id is sent: $(request /v1/oauth/token)"

echo exchange-fails >mode
run exchange-fails oidc env ANTHROPIC_API_KEY=sk-ant-api-test "$NARVI" summarize packages diff.md
[ "$(cat exchange-fails.out)" = "$summary" ] || fail "exchange-fails: no summary through the API key: $(cat exchange-fails.err)"
request /v1/messages | jq -e '."x-api-key" == "sk-ant-api-test" and .authorization == ""' >/dev/null ||
  fail "exchange-fails: the API key is not used after a refused exchange: $(request /v1/messages)"
grep -q "token exchange failed" exchange-fails.err || fail "exchange-fails: no warning: $(cat exchange-fails.err)"

echo ok >mode
run api-key env ANTHROPIC_API_KEY=sk-ant-api-test "$NARVI" summarize release diff.md
[ "$(cat api-key.out)" = "$summary" ] || fail "api-key: no summary: $(cat api-key.err)"
[ "$(request /v1/oauth/token)" = "" ] || fail "api-key: an exchange was tried without OIDC"
request /v1/messages | jq -e '."x-api-key" == "sk-ant-api-test" and .authorization == ""' >/dev/null || fail "api-key: wrong headers: $(request /v1/messages)"

run no-auth "$NARVI" summarize packages diff.md
[ ! -s no-auth.out ] || fail "no-auth: printed '$(cat no-auth.out)'"
[ ! -s requests.jsonl ] || fail "no-auth: a request was sent: $(cat requests.jsonl)"

for failure in error malformed empty; do
  echo "$failure" >mode
  run "$failure" env ANTHROPIC_API_KEY=sk-ant-api-test "$NARVI" summarize packages diff.md
  [ ! -s "$failure.out" ] || fail "$failure: printed '$(cat "$failure.out")'"
  grep -q "no summary" "$failure.err" || fail "$failure: no warning on stderr: $(cat "$failure.err")"
done

echo slow >mode
started=$(date +%s)
run slow env NARVI_TIMEOUT=2 ANTHROPIC_API_KEY=sk-ant-api-test "$NARVI" summarize packages diff.md
[ $(($(date +%s) - started)) -lt 8 ] || fail "slow: the timeout did not cut the call"
[ ! -s slow.out ] || fail "slow: printed '$(cat slow.out)'"

run slow-preface env NARVI_TIMEOUT=2 ANTHROPIC_API_KEY=sk-ant-api-test "$NARVI" preface report-green upstream diff.md 1234
[ "$(cat slow-preface.out)" = "$(printf '*%s*\n\n<!-- narvi -->' "$first")" ] || fail "slow-preface: the line alone is not kept: $(cat slow-preface.out)"

echo ok >mode
run preface env ANTHROPIC_API_KEY=sk-ant-api-test "$NARVI" preface report-green upstream diff.md 1234
expected=$(printf '*%s*\n\n**What this release brings to your fork**, summarized by Claude Haiku from the notes below:\n\n%s\n\n<!-- narvi -->' "$first" "$summary")
[ "$(cat preface.out)" = "$expected" ] || fail "preface: got '$(cat preface.out)'"

run silent "$NARVI" preface report-red upstream diff.md
[ ! -s silent.out ] || fail "silent: no line and no summary printed '$(cat silent.out)'"

head -c 100000 /dev/zero | tr '\0' 'x' >big.md
run big env ANTHROPIC_API_KEY=sk-ant-api-test "$NARVI" summarize packages big.md
request /v1/messages | jq -e '(.body.messages[0].content | length) < 26000 and (.body.messages[0].content | test("cut at 24000 bytes"))' >/dev/null ||
  fail "big: the input is not cut"

echo hostile >mode
run hostile env ANTHROPIC_API_KEY=sk-ant-api-test "$NARVI" summarize packages diff.md
zwsp=$(printf '\342\200\213')
[ "$(cat hostile.out)" = "x \\!-- @${zwsp}user a i https:/${zwsp}/evil.example \\| \\# done" ] || fail "hostile: the answer is not flattened: $(cat hostile.out)"
[ "$(wc -l <hostile.out)" -eq 1 ] || fail "hostile: the summary spans several lines"
if grep -qF -e '<!--' -e '](' -e '@user' -e '://' -e '[' hostile.out; then fail "hostile: a comment, a link, an image or a mention survived: $(cat hostile.out)"; fi

echo slow-exchange >mode
started=$(date +%s)
run slow-exchange oidc env NARVI_TIMEOUT=4 "$NARVI" summarize packages diff.md
[ $(($(date +%s) - started)) -lt 6 ] || fail "slow-exchange: the calls together took more than one budget"
[ ! -s slow-exchange.out ] || fail "slow-exchange: printed '$(cat slow-exchange.out)'"
request /v1/oauth/token | jq -e '.body.assertion == "jwt-from-github"' >/dev/null || fail "slow-exchange: no exchange"

echo error >mode
failed_cache=$PWD/failed-cache
run failed-first env NARVI_CACHE_DIR="$failed_cache" ANTHROPIC_API_KEY=sk-ant-api-test "$NARVI" summarize packages diff.md
[ -s requests.jsonl ] || fail "failed-first: the API was not called"
run failed-second env NARVI_CACHE_DIR="$failed_cache" ANTHROPIC_API_KEY=sk-ant-api-test "$NARVI" summarize packages diff.md
[ ! -s requests.jsonl ] || fail "failed-second: a failed input was sent again"
[ ! -s failed-second.out ] || fail "failed-second: printed '$(cat failed-second.out)'"
grep -q "earlier call for the same input failed" failed-second.err || fail "failed-second: no warning: $(cat failed-second.err)"

echo ok >mode
cache=$PWD/cache
run cached-first env NARVI_CACHE_DIR="$cache" ANTHROPIC_API_KEY=sk-ant-api-test "$NARVI" summarize packages diff.md
run cached-second env NARVI_CACHE_DIR="$cache" ANTHROPIC_API_KEY=sk-ant-api-test "$NARVI" summarize packages diff.md
[ "$(cat cached-second.out)" = "$summary" ] || fail "cached: the second call lost the summary"
[ ! -s requests.jsonl ] || fail "cached: the second call asked the API again"

export GH_STATE=$PWD/gh
mkdir -p "$GH_STATE"
original=$'Flake lock file updates:\n\n* Updated input nixpkgs: 2026-10-01 to 2026-10-12\n\n```\nkeep `this` exactly\n```'
printf '%s\n' "$original" >"$GH_STATE/pr-17.md"
printf '%s\n' "$original" >"$GH_STATE/release-v0.9.0.md"

expect_edited() {
  local file=$1 heading=$2 lore
  lore=$(head -n 1 "$file")
  [[ $lore =~ ^\*[^*]+\*$ ]] || fail "$file: does not open with a line of Narvi: $(cat "$file")"
  jq -e --arg line "${lore:1:-1}" 'any(.[][]; . == $line)' "$MESSAGES" >/dev/null || fail "$file: '$lore' is not a line of messages.json"
  grep -qxF "**$heading**, summarized by Claude Haiku from the notes below:" "$file" || fail "$file: no summary heading: $(cat "$file")"
  grep -qxF "$summary" "$file" || fail "$file: no summary: $(cat "$file")"
  [ "$(tail -n "$(wc -l <<<"$original")" "$file")" = "$original" ] || fail "$file: the original text is not intact below the preface: $(cat "$file")"
  [ "$(grep -c -- '<!-- narvi -->' "$file")" -eq 1 ] || fail "$file: not exactly one marker"
}

echo ok >mode
run pr env NARVI_MESSAGES="$MESSAGES" ANTHROPIC_API_KEY=sk-ant-api-test "$NARVI_FAKE_GH" pr lock-pr inputs 17
expect_edited "$GH_STATE/pr-17.md" "What this update brings"
request /v1/messages | jq -e '.body.messages[0].content | test("keep `this` exactly")' >/dev/null || fail "pr: the PR body was not summarized"
edited=$(cat "$GH_STATE/pr-17.md")
run pr-again env NARVI_MESSAGES="$MESSAGES" ANTHROPIC_API_KEY=sk-ant-api-test "$NARVI_FAKE_GH" pr lock-pr inputs 17
[ "$(cat "$GH_STATE/pr-17.md")" = "$edited" ] || fail "pr-again: a second run changed the body"
[ "$(grep -c '^pr edit' "$GH_STATE/calls")" -eq 1 ] || fail "pr-again: the body was edited again: $(cat "$GH_STATE/calls")"
[ ! -s requests.jsonl ] || fail "pr-again: the API was called for a body Narvi already opened"

run release env NARVI_MESSAGES="$MESSAGES" ANTHROPIC_API_KEY=sk-ant-api-test "$NARVI_FAKE_GH" release v0.9.0
expect_edited "$GH_STATE/release-v0.9.0.md" "What this release means for your machine"
grep -qx "release edit v0.9.0 --notes-file .*" "$GH_STATE/calls" || fail "release: notes not edited: $(cat "$GH_STATE/calls")"

printf '%s\n' "$original" >"$GH_STATE/pr-18.md"
run pr-no-auth env NARVI_MESSAGES="$MESSAGES" "$NARVI_FAKE_GH" pr upstream-pr upstream 18
[[ $(head -n 1 "$GH_STATE/pr-18.md") =~ ^\*[^*]+\*$ ]] || fail "pr-no-auth: no line of Narvi: $(cat "$GH_STATE/pr-18.md")"
if grep -q "summarized by Claude Haiku" "$GH_STATE/pr-18.md"; then fail "pr-no-auth: a summary heading without a summary"; fi
[ "$(tail -n "$(wc -l <<<"$original")" "$GH_STATE/pr-18.md")" = "$original" ] || fail "pr-no-auth: the original text is not intact"

printf '%s\n' "$original" >"$GH_STATE/pr-19.md"
: >"$GH_STATE/calls"
run pr-silent "$NARVI_FAKE_GH" pr lock-pr inputs 19
[ "$(cat "$GH_STATE/pr-19.md")" = "$original" ] || fail "pr-silent: a body was changed with nothing to say"
if grep -q '^pr edit' "$GH_STATE/calls"; then fail "pr-silent: the body was edited with nothing to say"; fi

touch "$GH_STATE/fail"
printf '%s\n' "$original" >"$GH_STATE/pr-20.md"
run pr-gh-fails env NARVI_MESSAGES="$MESSAGES" ANTHROPIC_API_KEY=sk-ant-api-test "$NARVI_FAKE_GH" pr lock-pr inputs 20
grep -q "cannot read pull request 20" pr-gh-fails.err || fail "pr-gh-fails: no warning: $(cat pr-gh-fails.err)"
run release-gh-fails env NARVI_MESSAGES="$MESSAGES" "$NARVI_FAKE_GH" release v1.0.0
grep -q "cannot read release v1.0.0" release-gh-fails.err || fail "release-gh-fails: no warning: $(cat release-gh-fails.err)"
rm "$GH_STATE/fail"

touch "$GH_STATE/fail-edit"
printf '%s\n' "$original" >"$GH_STATE/pr-21.md"
run pr-edit-fails env NARVI_MESSAGES="$MESSAGES" "$NARVI_FAKE_GH" pr lock-pr inputs 21
grep -q "cannot edit pull request 21" pr-edit-fails.err || fail "pr-edit-fails: no warning: $(cat pr-edit-fails.err)"
[ "$(cat "$GH_STATE/pr-21.md")" = "$original" ] || fail "pr-edit-fails: the body changed"
rm "$GH_STATE/fail-edit"

echo "narvi: all scenarios passed"
