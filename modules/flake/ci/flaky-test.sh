fail() {
  echo "FAIL: $*" >&2
  exit 1
}

export STATE=$PWD/state
mkdir -p "$STATE"

for check in steady flaky nondeterministic late broken; do
  "$CI_FLAKY" x86_64-linux "$check" 5 results >"$check.log" 2>&1 || fail "ci-flaky exits non-zero on $check: $(cat "$check.log")"
done

runs_of() {
  jq -c '[.runs[].passed]' "results/x86_64-linux-$1.json"
}

[ "$(runs_of steady)" = '[true,true,true,true,true]' ] || fail "steady runs: $(runs_of steady)"
[ "$(runs_of flaky)" = '[true,true,false,true,true]' ] || fail "a failed run stops or hides the others: $(runs_of flaky)"
[ "$(runs_of nondeterministic)" = '[true,true,true,true,true]' ] || fail "an output that differs counts as a failure: $(runs_of nondeterministic)"
[ "$(runs_of late)" = '[true,true,true,true,true]' ] || fail "a failed realise is counted as a run: $(runs_of late)"
[ "$(runs_of broken)" = '[false,false,false,false,false]' ] || fail "broken runs: $(runs_of broken)"

jq -e '.check == "flaky" and .system == "x86_64-linux" and (.runs | length == 5) and all(.runs[]; .seconds | type == "number" and . >= 0 and . == floor)' results/x86_64-linux-flaky.json >/dev/null ||
  fail "the result does not follow the contract: $(cat results/x86_64-linux-flaky.json)"

[ ! -e "$STATE/rebuilt-before-realise" ] || fail "--rebuild was called before a realise: $(cat "$STATE/rebuilt-before-realise")"
[ "$(grep -c -- '--rebuild' "$STATE/calls")" -eq 19 ] || fail "every run after a realise is a --rebuild: $(cat "$STATE/calls")"
[ "$(grep -c 'checks.x86_64-linux.broken$' "$STATE/calls")" -eq 6 ] || fail "broken is not built once to realise and once per run: $(cat "$STATE/calls")"

status=0
"$CI_FLAKY" x86_64-linux steady 0 results >zero.log 2>&1 || status=$?
[ "$status" -eq 2 ] || fail "0 runs exits with $status instead of 2"

commit() {
  git -C repo -c user.name=t -c user.email=t@t commit -q "$@"
}

git init -q --bare origin.git
git init -q -b main repo
echo source >repo/flake.nix
git -C repo add flake.nix
commit -m "main"
git -C repo remote add origin "$PWD/origin.git"
git -C repo push -q origin main

job() {
  jq -n -c --arg check "$1" '{ check: $check, system: "x86_64-linux", runner: "ubuntu-24.04" }'
}

evaluated() {
  jq -n -c --arg attr "$1" --arg drv "$2" '{ attr: $attr, drvPath: ("/nix/store/" + $drv + ".drv") }'
}

{
  job steady
  job flaky
  job changed
  job new
} | jq -s . >matrix.json
export MATRIX_FILE=$PWD/matrix.json

{
  evaluated x86_64-linux.new n1
  evaluated x86_64-linux.treefmt t1
  evaluated x86_64-linux.changed c2
  evaluated x86_64-linux.flaky f1
  evaluated x86_64-linux.steady s1
  evaluated aarch64-linux.steady a1
} >evaluated.jsonl
export EVALUATED_FILE=$PWD/evaluated.jsonl

record() {
  jq -n -c --arg drv "$1" --arg check "$2" --argjson passed "$3" \
    '{ key: ("/nix/store/" + $drv + ".drv"), value: { check: $check, system: "x86_64-linux", runs: ($passed | map({ passed: ., seconds: 7 })) } }'
}

result() {
  mkdir -p "$1"
  jq -n --arg check "$2" --argjson runs "$3" '{ check: $check, system: "x86_64-linux", runs: $runs }' >"$1/x86_64-linux-$2.json"
}

publish() {
  (cd repo && "$CI_BADGES" publish "../$1") >"$1.publish" 2>&1 || fail "publishing $1 fails: $(cat "$1.publish")"
}

plan() {
  local name=$1
  : >"$STATE/calls"
  (cd repo && "$CI_FLAKY_PLAN") >"$name.out" 2>"$name.err" || fail "$name: ci-flaky-plan exits non-zero: $(cat "$name.err")"
}

output() {
  sed -n "s/^$2=//p" "$1.out"
}

planned() {
  output "$1" checks | jq -r 'map(.check) | sort | join(" ")'
}

report() {
  local status=0 matrix planned
  matrix=$(output "$1" all)
  planned=$(output "$1" checks)
  (cd repo && MATRIX=$matrix PLANNED=$planned "$CI_FLAKY_REPORT" "../$1-results" "../$1-history") >"$1.md" 2>"$1.err" || status=$?
  echo "$status"
}

plan upstream
[ "$(planned upstream)" = "changed flaky new steady" ] || fail "upstream without history plans $(planned upstream)"
output upstream checks | jq -e 'all(.[]; .runs == 5 and .runner == "ubuntu-24.04")' >/dev/null || fail "a planned check lacks its runs or runner: $(cat upstream.out)"
[ "$(output upstream all | jq length)" -eq 4 ] || fail "all does not list every VM check: $(cat upstream.out)"

git -C repo remote set-url origin "$PWD/unreachable.git"
status=0
(cd repo && "$CI_FLAKY_PLAN") >unreachable.out 2>unreachable.err || status=$?
[ "$status" -ne 0 ] || fail "an origin that cannot be fetched plans every check: $(cat unreachable.out)"
grep -q 'could not be fetched' unreachable.err || fail "a failed fetch does not say so: $(cat unreachable.err)"
! grep -q 'no valid flaky.json' unreachable.err || fail "a failed fetch is reported as a missing record: $(cat unreachable.err)"
status=0
matrix=$(output upstream all)
(cd repo && MATRIX=$matrix PLANNED='[]' "$CI_FLAKY_REPORT" ../unreachable-results ../unreachable-history) >unreachable.md 2>&1 || status=$?
[ "$status" -eq 1 ] || fail "a report without the record exits $status instead of 1: $(cat unreachable.md)"
grep -q 'could not be fetched' unreachable.md || fail "the report does not say the fetch failed: $(cat unreachable.md)"
[ ! -e unreachable-history/flaky.json ] || fail "a report without the record writes a history that drops it: $(cat unreachable-history/flaky.json)"
git -C repo remote set-url origin "$PWD/origin.git"

mkdir malformed
echo 'not json' >malformed/flaky.json
publish malformed
plan malformed
[ "$(planned malformed)" = "changed flaky new steady" ] || fail "a malformed flaky.json plans $(planned malformed)"

mkdir shape
jq -n --argjson steady "$(record s1 steady '[true,true,true,true,true]')" '{ tree: 3, checks: ([$steady] | from_entries) }' >shape/flaky.json
publish shape
plan shape
[ "$(planned shape)" = "changed flaky new steady" ] || fail "a flaky.json of the wrong shape plans $(planned shape)"

mkdir first
{
  record s1 steady '[true,true,true,true,true]'
  record f1 flaky '[true,true,false,true,true]'
  record c1 changed '[true,true,true,true,true]'
} | jq -s '{ tree: null, checks: from_entries }' >first/flaky.json
publish first
plan night1
[ "$(planned night1)" = "changed flaky new" ] || fail "an unchanged passing derivation is not skipped, or a changed one is: $(planned night1)"

result night1-results flaky '[{"passed":true,"seconds":5},{"passed":true,"seconds":1},{"passed":false,"seconds":4},{"passed":true,"seconds":2},{"passed":true,"seconds":3}]'
result night1-results changed "$(jq -n -c '[10, 60, 20, 50, 30, 40] | map({ passed: true, seconds: . })')"
status=$(report night1)
[ "$status" -ne 0 ] || fail "a flaky check and a missing result exit 0: $(cat night1.md)"
grep -qxF '| `steady` | x86_64-linux | 100 % (not rerun, unchanged) | 7 s |' night1.md || fail "no recorded row for steady: $(cat night1.md night1.err)"
grep -qxF '| `flaky` | x86_64-linux | 80 % | 3 s |' night1.md || fail "no 80 % row with an odd median for flaky: $(cat night1.md)"
grep -qxF '| `changed` | x86_64-linux | 100 % | 35 s |' night1.md || fail "no row with an even median for changed: $(cat night1.md)"
grep -qxF '| `new` | x86_64-linux | no result | |' night1.md || fail "no row for a check without result: $(cat night1.md)"
grep -qxF '`flaky` passed 4 of its 5 runs.' night1.md || fail "no sentence for flaky: $(cat night1.md)"
grep -qF '`new` wrote no result' night1.md || fail "no sentence for new: $(cat night1.md)"
! grep -q 'Every VM check passed' night1.md || fail "the all-pass sentence is printed with a flaky check: $(cat night1.md)"
[ "$(jq -r '.checks | keys | join(" ")' night1-history/flaky.json)" = "/nix/store/c2.drv /nix/store/f1.drv /nix/store/s1.drv" ] ||
  fail "the history does not hold the current derivations only: $(cat night1-history/flaky.json)"
jq -e '.tree == null' night1-history/flaky.json >/dev/null || fail "a failing night records its tree: $(cat night1-history/flaky.json)"

publish night1-history
plan night2
[ "$(planned night2)" = "flaky new" ] || fail "a failing or missing derivation is not measured again: $(planned night2)"

result night2-results flaky "$(jq -n -c '[range(5)] | map({ passed: true, seconds: 2 })')"
result night2-results new "$(jq -n -c '[range(5)] | map({ passed: true, seconds: 9 })')"
status=$(report night2)
[ "$status" -eq 0 ] || fail "an all-green night exits $status: $(cat night2.md night2.err)"
grep -qxF 'Every VM check passed all of its runs.' night2.md || fail "no all-pass sentence: $(cat night2.md)"
[ "$(jq -r .tree night2-history/flaky.json)" = "$(git -C repo rev-parse 'HEAD^{tree}')" ] || fail "an all-green night does not record its tree: $(cat night2-history/flaky.json)"

publish night2-history
plan quiet
[ "$(output quiet checks)" = '[]' ] && [ "$(output quiet all)" = '[]' ] || fail "a measured tree plans checks: $(cat quiet.out)"
! grep -q eval "$STATE/calls" || fail "a measured tree evaluates the flake"

commit --allow-empty -m "same tree"
plan same-tree
[ "$(output same-tree all)" = '[]' ] || fail "a new commit with the same tree plans checks: $(cat same-tree.out)"

echo docs >repo/README.md
git -C repo add README.md
commit -m "docs"
plan docs
grep -q eval "$STATE/calls" || fail "a new tree is not evaluated"
[ "$(output docs checks)" = '[]' ] || fail "unchanged derivations are rerun: $(cat docs.out)"
[ "$(output docs all | jq length)" -eq 4 ] || fail "a new tree does not list every VM check for the report: $(cat docs.out)"
status=$(report docs)
[ "$status" -eq 0 ] || fail "a night with nothing rerun exits $status: $(cat docs.md docs.err)"
[ "$(grep -c '(not rerun, unchanged)' docs.md)" -eq 4 ] || fail "skipped checks are not all listed as unchanged: $(cat docs.md)"
[ "$(jq -r .tree docs-history/flaky.json)" = "$(git -C repo rev-parse 'HEAD^{tree}')" ] || fail "the new tree is not recorded: $(cat docs-history/flaky.json)"

echo "ci-flaky: all assertions passed"
