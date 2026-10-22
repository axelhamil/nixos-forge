fail() {
  echo "FAIL: $*" >&2
  exit 1
}

expect_badge() {
  local file=$1 message=$2 color=$3
  [ -f "$file" ] || fail "$file was not written"
  jq -e --arg message "$message" --arg color "$color" \
    '.schemaVersion == 1 and (.label | type == "string") and .message == $message and .color == $color' "$file" >/dev/null ||
    fail "$file: expected '$message' in $color, got $(cat "$file")"
}

pass_all() {
  mkdir -p "$1"
  jq -c '.[]' <<<"$PLANNED" | while read -r job; do
    jq '{ check, system, outcome: "passed", excerpt: [], bootTime: null, screenshots: [] }' <<<"$job" >"$1/$(jq -r '.system + "-" + .check' <<<"$job").json"
  done
}

set_field() {
  local file=$1 filter=$2
  jq "$filter" "$file" >"$file.tmp"
  mv "$file.tmp" "$file"
}

badges() {
  RESULTS_DIR=$1 "$CI_BADGES" checks "$1/badges" >/dev/null || fail "$1: ci-badges exits non-zero"
}

export PLANNED SKIPPED HOST_CHECKS LINT_RESULT
HOST_CHECKS=$(cat "$RUNS/host-checks.json")
PLANNED=$(cat "$RUNS/planned.json")
SKIPPED='[]'
LINT_RESULT=success

pass_all green
set_field green/x86_64-linux-example-boots.json '.bootTime = { total: 7.04, userspace: 3.0 }'
set_field green/x86_64-linux-example-server-boots.json '.bootTime = { total: 12, userspace: 5 }'
badges green
expect_badge green/badges/checks.json "12/12 passing" brightgreen
expect_badge green/badges/vm-tests.json 7 5277C3
expect_badge green/badges/hosts.json "3 built" 5277C3
expect_badge green/badges/boot-time.json "7.0 s" 5277C3

pass_all cached
rm cached/x86_64-linux-example-boots.json cached/x86_64-linux-feature-ssh.json
mkdir -p cached/current
echo '{ "example": { "total": 6.84, "userspace": 2.1 } }' >cached/current/boot-time.json
PLANNED=$(jq -c 'map(select(.check != "example-boots" and .check != "feature-ssh"))' "$RUNS/planned.json") \
  SKIPPED='[{"check":"example-boots","system":"x86_64-linux","vm":true,"reason":"cached"},{"check":"feature-ssh","system":"x86_64-linux","vm":true,"reason":"cached"}]' \
  badges cached
expect_badge cached/badges/checks.json "12/12 passing" brightgreen
expect_badge cached/badges/vm-tests.json 7 5277C3
expect_badge cached/badges/boot-time.json "6.8 s" 5277C3

pass_all red
set_field red/x86_64-linux-example-server-toplevel.json '.outcome = "failed"'
rm red/x86_64-linux-example-disk.json
LINT_RESULT=failure badges red
expect_badge red/badges/checks.json "3/12 failing" red
expect_badge red/badges/hosts.json "2/3 built" red
[ ! -e red/badges/boot-time.json ] || fail "red: a boot time is published without one recorded: $(cat red/badges/boot-time.json)"

mkdir -p no-plan
PLANNED='' SKIPPED='' HOST_CHECKS='' LINT_RESULT=failure badges no-plan
expect_badge no-plan/badges/checks.json "flake does not evaluate" red
[ "$(ls no-plan/badges)" = checks.json ] || fail "no-plan: badges other than checks are published: $(ls no-plan/badges)"

cat >identical.md <<'EOF'
## Reproducibility

- `example`: the rebuild is identical.
- `example-server`: the rebuild is identical.
EOF
"$CI_BADGES" reproducibility identical.md repro-identical >/dev/null || fail "reproducibility: exits non-zero"
expect_badge repro-identical/reproducibility.json reproducible brightgreen

cat >differs.md <<'EOF'
- `example`: the rebuild differs from the first build.

<details>
<summary>diffoscope of example, first 200 lines</summary>
</details>

- `example-server`: the rebuild differs from the first build.
- `example-desktop`: the rebuild is identical.
EOF
"$CI_BADGES" reproducibility differs.md repro-differs >/dev/null || fail "reproducibility: a difference exits non-zero"
expect_badge repro-differs/reproducibility.json "2 paths differ" orange

cat >broken.md <<'EOF'
- `example`: the rebuild is identical.
- `example-server`: the system did not build, see the job log.
EOF
"$CI_BADGES" reproducibility broken.md repro-broken >/dev/null || fail "reproducibility: a broken build exits non-zero"
expect_badge repro-broken/reproducibility.json "not finished" lightgrey

git init -q --bare origin.git
git init -q -b main repo
git -C repo -c user.name=t -c user.email=t@t commit -q --allow-empty -m "main"
echo source >repo/flake.nix
git -C repo add flake.nix
git -C repo -c user.name=t -c user.email=t@t commit -q -m "source"
git -C repo remote add origin "$PWD/origin.git"
git -C repo push -q origin main

(cd repo && "$CI_BADGES" publish ../green/badges) >publish.out || fail "publish: the first publish fails: $(cat publish.out)"
(cd repo && "$CI_BADGES" publish ../repro-differs) >>publish.out || fail "publish: the second publish fails: $(cat publish.out)"
files=$(git -C origin.git ls-tree --name-only badges | paste -sd ' ')
[ "$files" = "boot-time.json checks.json hosts.json reproducibility.json vm-tests.json" ] || fail "publish: the badges branch holds '$files'"
if git -C origin.git merge-base badges main >/dev/null; then fail "publish: the badges branch shares history with main"; fi
[ "$(git -C origin.git rev-list --count badges)" -eq 2 ] || fail "publish: expected 2 commits on badges"

(cd repo && "$CI_BADGES" publish ../repro-differs) >unchanged.out || fail "publish: an unchanged publish fails"
grep -q "did not change" unchanged.out || fail "publish: an unchanged publish is not reported: $(cat unchanged.out)"
[ "$(git -C origin.git rev-list --count badges)" -eq 2 ] || fail "publish: an unchanged publish adds a commit"
[ -z "$(git -C repo worktree list | sed 1d)" ] || fail "publish: a worktree is left behind: $(git -C repo worktree list)"

[ "$(cd repo && "$CI_BADGES" read checks.json)" = "$(cat green/badges/checks.json)" ] || fail "read: the published checks.json is not read back"
status=0
(cd repo && "$CI_BADGES" read missing.json) >/dev/null 2>&1 || status=$?
[ "$status" -eq 3 ] || fail "read: a file missing from the badges branch exits $status instead of 3"

git -C repo remote set-url origin "$PWD/unreachable.git"
status=0
(cd repo && "$CI_BADGES" read checks.json) >/dev/null 2>unreachable.err || status=$?
[ "$status" -eq 1 ] || fail "read: a failed fetch exits $status instead of 1"
grep -q 'could not be fetched' unreachable.err || fail "read: a failed fetch does not say so: $(cat unreachable.err)"
status=0
(cd repo && "$CI_BADGES" publish ../green/badges) >/dev/null 2>&1 || status=$?
[ "$status" -eq 1 ] || fail "publish: a failed fetch exits $status instead of starting an orphan badges branch"

echo "ci-badges: all assertions passed"
