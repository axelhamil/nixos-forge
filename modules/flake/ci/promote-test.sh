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
grep -qx "unsigned: DEPLOY_SIGNING_KEY is not set, so deploy branches point to main's own commits, which hosts with update.signers refuse" first.out || fail "first: no line saying the promotion is unsigned: $(cat first.out)"
[ "$(wc -l <first.out)" -eq 4 ] || fail "first: not one line per host: $(cat first.out)"

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
rm ../origin.git/hooks/pre-receive

cd ..
ssh-keygen -q -t ed25519 -N "" -C deploy -f signing
ssh-keygen -q -t ed25519 -N "" -C other -f other
echo "deploy namespaces=\"git\" $(cat signing.pub)" >allowed
DEPLOY_SIGNING_KEY=$(cat signing)
export DEPLOY_SIGNING_KEY
git init -q --bare signed.git
cd work
git push -q ../signed.git main
git remote set-url origin ../signed.git

verified() {
  git -c gpg.format=ssh -c gpg.ssh.allowedSignersFile=../allowed verify-commit "$1" 2>/dev/null
}

parents() {
  git rev-list --parents -n 1 "$1" | cut -d' ' -f2-
}

signed_deploy() {
  git --git-dir=../signed.git rev-parse --verify -q "refs/heads/deploy/$1" || true
}

results ../signed-first
status=0
"$CI_PROMOTE" ../signed-first "$c" >signed-first.out 2>&1 || status=$?
[ "$status" -eq 0 ] || fail "a signed promotion of C exits with $status: $(cat signed-first.out)"
git fetch -q origin '+refs/heads/deploy/*:refs/remotes/origin/deploy/*'
p1=$(signed_deploy example)
[ -n "$p1" ] && [ "$p1" != "$c" ] || fail "deploy/example is not a promotion commit of its own: $p1"
verified "$p1" || fail "the promotion commit of deploy/example does not verify with the signing key"
[ "$(parents "$p1")" = "$c" ] || fail "the first promotion has the parents $(parents "$p1"), expected C alone"
[ "$(git rev-parse "$p1^{tree}")" = "$(git rev-parse "$c^{tree}")" ] || fail "the promotion commit does not carry C's tree"
[ "$(git log -1 --format=%s "$p1")" = "deploy example: checks passed for ${c:0:7}" ] || fail "the promotion subject is $(git log -1 --format=%s "$p1")"
grep -qx "deploy/example -> ${c:0:7}, signed as ${p1:0:7}" signed-first.out || fail "signed: no promotion line for example: $(cat signed-first.out)"
if grep -q unsigned signed-first.out; then
  fail "a signed promotion says it is unsigned: $(cat signed-first.out)"
fi

f=$(commit F)
git push -q origin main
status=0
"$CI_PROMOTE" ../signed-first "$f" >signed-second.out 2>&1 || status=$?
[ "$status" -eq 0 ] || fail "a second signed promotion exits with $status: $(cat signed-second.out)"
git fetch -q origin '+refs/heads/deploy/*:refs/remotes/origin/deploy/*'
p2=$(signed_deploy example)
verified "$p2" || fail "the second promotion commit does not verify"
[ "$(parents "$p2")" = "$p1 $f" ] || fail "the second promotion has the parents $(parents "$p2"), expected the previous promotion then F"
[ "$(git rev-parse "$p2^{tree}")" = "$(git rev-parse "$f^{tree}")" ] || fail "the second promotion does not carry F's tree"
git merge-base --is-ancestor "$p1" "$p2" || fail "deploy/example moved without descending from its previous promotion"

status=0
"$CI_PROMOTE" ../signed-first "$c" >signed-back.out 2>&1 || status=$?
[ "$status" -eq 0 ] || fail "promoting an older commit exits with $status: $(cat signed-back.out)"
grep -qx "deploy/example already at or past ${c:0:7}" signed-back.out || fail "signed: no already line for an older commit: $(cat signed-back.out)"
[ "$(signed_deploy example)" = "$p2" ] || fail "deploy/example moved back"

status=0
"$CI_PROMOTE" ../signed-first "$d" >signed-sideways.out 2>&1 || status=$?
[ "$status" -eq 1 ] || fail "a sideways promotion exits with $status instead of 1: $(cat signed-sideways.out)"
grep -qx "deploy/example refused: not a fast-forward" signed-sideways.out || fail "signed: no refused line for a commit that does not descend from F: $(cat signed-sideways.out)"
[ "$(signed_deploy example)" = "$p2" ] || fail "deploy/example moved sideways"

forged=$(git commit-tree -p "$p2" -p "$d" -m "deploy example: checks passed for ${d:0:7}" "$d^{tree}")
git push -q origin "+$forged:refs/heads/deploy/example"
status=0
"$CI_PROMOTE" ../signed-first "$f" >signed-forged.out 2>&1 || status=$?
[ "$status" -eq 0 ] || fail "a promotion over an unsigned tip exits with $status: $(cat signed-forged.out)"
grep -qx "deploy/example warning: its tip ${forged:0:7} is not a promotion signed by DEPLOY_SIGNING_KEY, so the promotion replaces it instead of building on it" signed-forged.out || fail "signed: no warning for a forged tip: $(cat signed-forged.out)"
p3=$(signed_deploy example)
verified "$p3" || fail "the promotion replacing a forged tip does not verify"
[ "$(parents "$p3")" = "$f" ] || fail "the promotion replacing a forged tip has the parents $(parents "$p3"), expected F alone"

git push -q origin "+$f:refs/heads/deploy/example"
status=0
"$CI_PROMOTE" ../signed-first "$f" >signed-reset.out 2>&1 || status=$?
[ "$status" -eq 0 ] || fail "a promotion over a tip reset to main exits with $status: $(cat signed-reset.out)"
if grep -q "^deploy/example already at or past" signed-reset.out; then
  fail "an unsigned tip counts as already promoted: $(cat signed-reset.out)"
fi
p4=$(signed_deploy example)
verified "$p4" || fail "the promotion over a tip reset to F does not verify"
[ "$(parents "$p4")" = "$f" ] || fail "the promotion over a tip reset to F has the parents $(parents "$p4"), expected F alone"

DEPLOY_SIGNING_KEY="not a key"
h=$(commit H)
git push -q origin main
status=0
"$CI_PROMOTE" ../signed-first "$h" >signed-broken.out 2>&1 || status=$?
[ "$status" -eq 1 ] || fail "a key that cannot sign exits with $status instead of 1: $(cat signed-broken.out)"
grep -qx "deploy/example refused: DEPLOY_SIGNING_KEY cannot sign, it must hold an OpenSSH private key without a passphrase" signed-broken.out || fail "signed: no line for a key that cannot sign: $(cat signed-broken.out)"
[ "$(signed_deploy example)" = "$p4" ] || fail "deploy/example moved without a signature"
if grep -q "not a key" signed-broken.out; then
  fail "ci-promote printed the content of DEPLOY_SIGNING_KEY"
fi

DEPLOY_SIGNING_KEY=$(cat ../other)
g=$(commit G)
git push -q origin main
status=0
"$CI_PROMOTE" ../signed-first "$g" >signed-other.out 2>&1 || status=$?
[ "$status" -eq 0 ] || fail "a promotion signed by another key exits with $status: $(cat signed-other.out)"
git fetch -q origin '+refs/heads/deploy/*:refs/remotes/origin/deploy/*'
if verified "$(signed_deploy example)"; then
  fail "a promotion signed by a key absent from the allowed signers verifies"
fi

echo "ci-promote: all scenarios passed"
