fail() {
  echo "FAIL: $*" >&2
  exit 1
}

export HOME=$PWD/home GIT_AUTHOR_NAME=test GIT_AUTHOR_EMAIL=test@example.com GIT_COMMITTER_NAME=test GIT_COMMITTER_EMAIL=test@example.com
mkdir -p "$HOME"
git config --global init.defaultBranch main
git config --global advice.detachedHead false

git init -q --bare origin.git
git clone -q origin.git work 2>/dev/null
cd work

commit() {
  echo "$1" >file
  git add file
  git commit -q -m "$1"
  git rev-parse HEAD
}

a=$(commit A)
b=$(commit B)
c=$(commit C)
git push -q origin main

deploy() {
  git --git-dir=../origin.git rev-parse --verify -q "refs/heads/deploy/$1" || true
}

export HOST_CHECKS PLANNED SKIPPED
HOST_CHECKS=$(cat "$RUNS/host-checks.json")
PLANNED=$(jq -c 'map(select(.check != "example-toplevel" and .check != "example-pi-sd-image"))' "$RUNS/planned.json")
SKIPPED='[{"check":"example-toplevel","system":"x86_64-linux","reason":"cached"},{"check":"example-pi-sd-image","system":"aarch64-linux","reason":"compiles a kernel"}]'

results() {
  local dir=$1
  shift
  rm -rf "$dir"
  mkdir -p "$dir"
  jq -c '.[]' <<<"$PLANNED" | while read -r job; do
    jq '{ check, system, outcome: "passed", attempts: 1, excerpt: [] }' <<<"$job" >"$dir/$(jq -r '.system + "-" + .check' <<<"$job").json"
  done
  while [ $# -gt 0 ]; do
    if [ "$2" = missing ]; then
      rm "$dir/x86_64-linux-$1.json"
    else
      jq --arg outcome "$2" '.outcome = $outcome' "$dir/x86_64-linux-$1.json" >"$dir/tmp"
      mv "$dir/tmp" "$dir/x86_64-linux-$1.json"
    fi
    shift 2
  done
}

results ../first example-boots unstable example-server-boots failed
status=0
"$CI_PROMOTE" ../first "$b" >first.out 2>&1 || status=$?
[ "$status" -eq 0 ] || fail "promoting B exits with $status: $(cat first.out)"
[ "$(deploy example)" = "$b" ] || fail "deploy/example is not B after its checks passed: $(cat first.out)"
[ -z "$(deploy example-server)" ] || fail "deploy/example-server moved although example-server-boots failed"
[ -z "$(deploy example-pi)" ] || fail "deploy/example-pi moved although its check was skipped for a kernel"
grep -qx "deploy/example -> ${b:0:7}" first.out || fail "first: no promotion line for example: $(cat first.out)"
grep -qx "deploy/example-server kept: example-server-boots failed" first.out || fail "first: no kept line for example-server: $(cat first.out)"
grep -qx "deploy/example-pi kept: example-pi-sd-image skipped" first.out || fail "first: no kept line for example-pi: $(cat first.out)"
[ "$(wc -l <first.out)" -eq 3 ] || fail "first: not one line per host: $(cat first.out)"

results ../second example-disk missing
status=0
"$CI_PROMOTE" ../second "$c" >second.out 2>&1 || status=$?
[ "$status" -eq 0 ] || fail "a missing result exits with $status: $(cat second.out)"
[ "$(deploy example)" = "$b" ] || fail "deploy/example moved although example-disk did not finish"
[ "$(deploy example-server)" = "$c" ] || fail "deploy/example-server is not C after its checks passed: $(cat second.out)"
grep -qx "deploy/example kept: example-disk did not finish" second.out || fail "second: no kept line for example: $(cat second.out)"

git checkout -q --orphan elsewhere
d=$(commit D)
git push -q origin "+$d:refs/heads/deploy/example"
git checkout -q main

results ../third
status=0
"$CI_PROMOTE" ../third "$c" >third.out 2>&1 || status=$?
[ "$status" -eq 1 ] || fail "a refused push exits with $status instead of 1: $(cat third.out)"
[ "$(deploy example)" = "$d" ] || fail "deploy/example was rewritten although C does not descend from D"
grep -qx "deploy/example refused: not a fast-forward" third.out || fail "third: no refused line: $(cat third.out)"
grep -qx "deploy/example-server already at or past ${c:0:7}" third.out || fail "third: a refusal stopped the other hosts, or the same commit is not already there: $(cat third.out)"

git push -q origin "+$b:refs/heads/deploy/example"
status=0
"$CI_PROMOTE" ../third "$a" >back.out 2>&1 || status=$?
[ "$status" -eq 0 ] || fail "promoting an ancestor of every deploy branch exits with $status: $(cat back.out)"
grep -qx "deploy/example already at or past ${a:0:7}" back.out || fail "back: no already line for example: $(cat back.out)"
[ "$(deploy example-server)" = "$c" ] || fail "deploy/example-server moved back to A"
grep -qx "deploy/example-server already at or past ${a:0:7}" back.out || fail "back: no already line: $(cat back.out)"

e=$(commit E)
printf '#!/bin/sh\necho "remote: deploy branches are frozen" >&2\nexit 1\n' >../origin.git/hooks/pre-receive
chmod +x ../origin.git/hooks/pre-receive
status=0
"$CI_PROMOTE" ../third "$e" >hook.out 2>&1 || status=$?
[ "$status" -eq 1 ] || fail "a push declined by the remote exits with $status instead of 1: $(cat hook.out)"
[ "$(deploy example-server)" = "$c" ] || fail "deploy/example-server moved although the remote declined it"
grep -qx "deploy/example-server refused: \[remote rejected\] (pre-receive hook declined)" hook.out || fail "hook: the refusal reason is lost: $(cat hook.out)"

echo "ci-promote: all scenarios passed"
