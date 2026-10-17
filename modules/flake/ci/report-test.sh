fail() {
  echo "FAIL: $*" >&2
  exit 1
}

expect_excerpt() {
  local log=$1 pattern=$2 excerpt
  [ -f "$log" ] || log=$LOGS/$log
  excerpt=$(awk -f "$EXCERPT" "$log")
  grep -q -- "$pattern" <<<"$excerpt" || fail "$log: excerpt misses '$pattern': $excerpt"
  [ "$(wc -l <<<"$excerpt")" -le 20 ] || fail "$log: excerpt has more than 20 lines"
  if grep -q $'\033' <<<"$excerpt"; then fail "$log: excerpt keeps terminal escapes"; fi
  [ "$(sort <<<"$excerpt" | uniq -d)" = "" ] || fail "$log: excerpt repeats a line"
}

expect_excerpt assertion.log "^AssertionError: doctor misses a file written to /$"
expect_excerpt assertion.log "^Traceback (most recent call last):$"
expect_excerpt eval-error.log "^error: undefined variable 'foo'$"
expect_excerpt timeout.log "^RequestedAssertionFailed: action timed out after 120.73 seconds"

seq -f "machine # [%g] dhcpcd: eth0: DHCPv6 solicit timed out" 30 >noisy.log
echo "AssertionError: late failure" >>noisy.log
expect_excerpt noisy.log "^AssertionError: late failure$"
expect_excerpt compile-error.log "^main.c:3:1: error: expected ';' before '}' token$"
expect_excerpt compile-error.log "^error: Cannot build "

seq -f "machine # line %g of a healthy boot" 5000 >quiet.log
[ "$(awk -f "$EXCERPT" quiet.log | wc -l)" -eq 0 ] || fail "a log without failure yields an excerpt"

export NIX_CALLS

NIX_CALLS=$PWD/flaky-calls
if ! "$FLAKY_CI_BUILD" x86_64-linux example-boots results --retry >flaky.out; then
  fail "a vm test passing on its second attempt exits non-zero"
fi
result=results/x86_64-linux-example-boots.json
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
result=results/x86_64-linux-example-toplevel.json
jq -e '.outcome == "failed" and .attempts == 1' "$result" >/dev/null || fail "broken: not failed after 1 attempt: $(cat "$result")"
jq -e '.excerpt == ["error: undefined variable '"'"'foo'"'"'"]' "$result" >/dev/null || fail "broken: wrong excerpt: $(cat "$result")"
[ "$(wc -l <"$NIX_CALLS")" -eq 1 ] || fail "broken: a check without --retry was built $(wc -l <"$NIX_CALLS") times"

NIX_CALLS=$PWD/broken-vm-calls
status=0
"$BROKEN_CI_BUILD" x86_64-linux example-disk results --retry >broken-vm.out || status=$?
[ "$status" -eq 1 ] || fail "a vm test failing twice exits with $status instead of 1"
jq -e '.outcome == "failed" and .attempts == 2' results/x86_64-linux-example-disk.json >/dev/null || fail "broken vm: not failed after 2 attempts: $(cat results/x86_64-linux-example-disk.json)"

NIX_CALLS=$PWD/killed-calls
status=0
"$KILLED_CI_BUILD" x86_64-linux example-boots killed --retry >killed.out || status=$?
[ "$status" -ne 0 ] || fail "killed: a build killed by the time limit exits 0"
result=killed/x86_64-linux-example-boots.json
jq -e '.outcome == "failed" and .attempts == 1' "$result" >/dev/null || fail "killed: the first failure is not kept: $(cat "$result" 2>&1)"
jq -e '.excerpt | any(test("^RequestedAssertionFailed: action timed out"))' "$result" >/dev/null || fail "killed: the excerpt of the first failure is lost: $(cat "$result")"

export PLANNED SKIPPED CACHE_OUTCOME LINT_RESULT PLAN_RESULT RUN_URL HOST_CHECKS DESCRIPTIONS
PLAN_RESULT=success
HOST_CHECKS=$(cat "$RUNS/host-checks.json")
DESCRIPTIONS=$(cat "$DESCRIPTIONS_FILE")
PLANNED=$(cat "$RUNS/planned.json")
SKIPPED='[]'
CACHE_OUTCOME=success
LINT_RESULT=success
RUN_URL=https://github.com/owner/repo/actions/runs/42

pass_all() {
  mkdir -p "$1"
  jq -c '.[]' <<<"$PLANNED" | while read -r job; do
    jq '{ check, system, outcome: "passed", attempts: 1, excerpt: [] }' <<<"$job" >"$1/$(jq -r '.system + "-" + .check' <<<"$job").json"
  done
}

set_outcome() {
  local dir=$1 check=$2 outcome=$3 attempts=$4 log=${5:-} system=${6:-x86_64-linux}
  local excerpt='[]'
  local file=$dir/$system-$check.json
  [ -z "$log" ] || excerpt=$(awk -f "$EXCERPT" "$log" | jq -R . | jq -s -c .)
  jq --arg outcome "$outcome" --argjson attempts "$attempts" --argjson excerpt "$excerpt" \
    '.outcome = $outcome | .attempts = $attempts | .excerpt = $excerpt' "$file" >"$file.tmp"
  mv "$file.tmp" "$file"
}

report() {
  local dir=$1
  shift
  RESULTS_DIR=$dir "$CI_REPORT" "$@" >"$dir.md" || fail "$dir: ci-report exits non-zero"
  if LC_ALL=C grep -n '[^ -~]' "$dir.md"; then fail "$dir: the report has a character outside plain ASCII (dash, emoji)"; fi
}

expect_first_line() {
  [ "$(head -n 1 "$1.md")" = "$2" ] || fail "$1: first line is '$(head -n 1 "$1.md")', expected '$2'"
}

expect_line() {
  grep -qxF -- "$2" "$1.md" || fail "$1: no line '$2' in: $(cat "$1.md")"
}

section() {
  awk -v title="### $2" '/^(#|<)/ { on = ($0 == title); next } on && /^\| `/' "$1.md"
}

expect_rows() {
  local dir=$1 host=$2 expected=$3 rows
  rows=$(section "$dir" "$host" | awk -F '|' '{ gsub(/[ `]/, "", $2); gsub(/ /, "", $4); print $2 "=" $4 }' | paste -sd ' ')
  [ "$rows" = "$expected" ] || fail "$dir: section $host is '$rows', expected '$expected'"
}

pass_all all-green
printf '### Closure diff (x86_64-linux)\n\n#### example\n```\nnew host\n```\n' >all-green/closure-x86_64-linux.md
report all-green
expect_first_line all-green "**Everything works: every machine builds, boots and passed its tests.**"
expect_rows all-green example "example-toplevel=passed example-boots=passed example-disk=passed example-wipe-home=passed"
expect_rows all-green example-pi "example-pi-sd-image=passed"
expect_line all-green '| `example-server-installer` | the USB installer installs it | passed |'
expect_line all-green '| `example-disk` | it installs on an encrypted disk, wipes / at boot and restores a snapshot | passed |'
expect_line all-green '| `feature-ssh` | ssh works in a VM | passed |'
expect_line all-green '| `treefmt` | the code is formatted | passed |'
expect_line all-green "<summary>Module checks: 3 passed</summary>"
expect_line all-green "<summary>Package changes per machine</summary>"
expect_line all-green "new host"
if grep -q "What went wrong" all-green.md; then fail "all-green: a green run lists failures"; fi

pass_all one-failed
set_outcome one-failed example-server-boots failed 2 "$LOGS/assertion.log"
report one-failed
expect_first_line one-failed "**1 check failed: example-server would not get this change.**"
expect_rows one-failed example "example-toplevel=passed example-boots=passed example-disk=passed example-wipe-home=passed"
expect_rows one-failed example-server "example-server-toplevel=passed example-server-boots=failed example-server-disk=passed example-server-installer=passed"
expect_line one-failed '#### `example-server-boots` (x86_64-linux)'
expect_line one-failed "- Cause: the system started but a test found it behaving wrong."
expect_line one-failed '- What to do: read the failing line, then reproduce with `nix build -L .#checks.x86_64-linux.example-server-boots`.'
expect_line one-failed "AssertionError: doctor misses a file written to /"
expect_line one-failed "[Open the full log](https://github.com/owner/repo/actions/runs/42)"

pass_all unstable
set_outcome unstable example-boots unstable 2 "$LOGS/timeout.log"
report unstable
expect_first_line unstable "**Everything works: every machine builds, boots and passed its tests.**"
[ "$(sed -n 3p unstable.md)" = "1 test passed only on its second try and is marked unstable." ] || fail "unstable: line 3 is '$(sed -n 3p unstable.md)'"
expect_rows unstable example "example-toplevel=passed example-boots=unstable example-disk=passed example-wipe-home=passed"

pass_all missing
rm missing/x86_64-linux-example-disk.json
report missing
expect_first_line missing "**1 check did not finish: example would not get this change.**"
expect_rows missing example "example-toplevel=passed example-boots=passed example-disk=didnotfinish example-wipe-home=passed"
expect_line missing "- Cause: the job stopped before it wrote a result: it was cancelled or hit its time limit."

pass_all cache-down
CACHE_OUTCOME=failure report cache-down
expect_line cache-down "The binary cache was unreachable: everything was built from scratch."
if grep -q "binary cache was unreachable" all-green.md; then fail "all-green: a working cache is reported unreachable"; fi

pass_all kernel
rm kernel/aarch64-linux-example-pi-sd-image.json kernel/x86_64-linux-feature-ssh.json
SKIPPED='[{"check":"example-pi-sd-image","system":"aarch64-linux","reason":"compiles a kernel"},{"check":"feature-ssh","system":"x86_64-linux","reason":"cached"}]' \
  PLANNED=$(jq -c 'map(select(.check != "example-pi-sd-image" and .check != "feature-ssh"))' <<<"$PLANNED") \
  report kernel
expect_first_line kernel "**Everything works: every machine builds, boots and passed its tests.**"
expect_line kernel "Checks that compile a kernel run after the merge."
expect_line kernel "1 check was skipped: it already passed with the same code."
expect_rows kernel example-pi "example-pi-sd-image=skipped:runsafterthemerge"
expect_line kernel '| `feature-ssh` | ssh works in a VM | skipped: already passed |'

pass_all module-failed
set_outcome module-failed feature-ssh failed 1 "$LOGS/eval-error.log"
LINT_RESULT=failure report module-failed
expect_first_line module-failed "**2 checks failed: no machine is affected, but CI stays red until it is fixed.**"
expect_line module-failed "<details open>"
expect_line module-failed "- Cause: the configuration does not evaluate."
expect_line module-failed '- What to do: run `nix fmt`, then commit the result.'

pass_all timeout
set_outcome timeout example-boots failed 2 "$LOGS/timeout.log"
report timeout
expect_line timeout "- Cause: a VM test hit its time limit."

printf 'machine # [   12.0] systemd[1]: Started backup.service.\nRequestedAssertionFailed: command `test -e /persist/state` failed (exit code 1)\n' >assertion-only.log
pass_all assertion-only
set_outcome assertion-only example-boots failed 2 "$PWD/assertion-only.log"
report assertion-only
expect_line assertion-only "- Cause: the system started but a test found it behaving wrong."

printf 'machine # [    8.0] dhcpcd[612]: eth0: DHCPv6 solicit timed out\nAssertionError: the greeter never showed\n' >guest-noise.log
pass_all guest-noise
set_outcome guest-noise example-boots failed 2 "$PWD/guest-noise.log"
report guest-noise
expect_line guest-noise "- Cause: the system started but a test found it behaving wrong."

mkdir -p no-plan
PLANNED='' report no-plan
expect_first_line no-plan "**CI could not plan this run, so nothing was built.**"

mkdir -p plan-failed
PLANNED='' SKIPPED='' CACHE_OUTCOME='' HOST_CHECKS='' DESCRIPTIONS='' PLAN_RESULT=failure LINT_RESULT=failure report plan-failed
expect_first_line plan-failed "**The flake did not evaluate, so nothing was built or tested.**"
expect_line plan-failed 'The `plan` job log names the file and line: [open the run](https://github.com/owner/repo/actions/runs/42), then the `plan` job.'
if grep -q "Everything works" plan-failed.md; then fail "plan-failed: a flake that does not evaluate is reported as working"; fi

pass_all fence
printf '    > ````markdown\n    > ``x``\nerror: builder failed\n' >fence.log
set_outcome fence feature-ssh failed 1 "$PWD/fence.log"
report fence
expect_line fence "\`\`\`\`markdown"
expect_line fence "\`\`\`\`\`"
[ "$(grep -c '^`````$' fence.md)" -eq 2 ] || fail "fence: the excerpt fence is not longer than the backticks it holds: $(cat fence.md)"

pass_all escape
for name in 'system-a|b' 'app-x`y'; do
  jq -n --arg check "$name" '{ check: $check, system: "x86_64-linux", outcome: "passed", attempts: 1, excerpt: [] }' >"escape/x86_64-linux-$name.json"
done
PLANNED=$(jq -c '. + [{ check: "system-a|b", system: "x86_64-linux" }, { check: "app-x`y", system: "x86_64-linux" }]' <<<"$PLANNED") report escape
expect_line escape '| `system-a\|b` | its own test passes | passed |'
expect_line escape '| ``app-x`y`` | its own test passes | passed |'

pass_all compile
set_outcome compile example-toplevel failed 1 "$LOGS/compile-error.log"
report compile
expect_line compile "- Cause: a package or test failed to build."
expect_line compile '- What to do: read the lines above `Cannot build` in the full log.'
expect_line compile "main.c:3:1: error: expected ';' before '}' token"

pass_all hash
set_outcome hash example-toplevel failed 1 "$LOGS/hash-mismatch.log"
report hash
expect_line hash "- Cause: a download changed upstream."

printf 'error: writing to file: No space left on device\nerror: Cannot build '"'"'/nix/store/x-example.drv'"'"'.\n' >no-space.log
pass_all no-space
set_outcome no-space example-toplevel failed 1 "$PWD/no-space.log"
report no-space
expect_line no-space "- Cause: the CI runner ran out of disk."

two_systems=$(jq -c '. + [{ check: "feature-ssh", system: "aarch64-linux" }]' <<<"$PLANNED")
PLANNED=$two_systems pass_all two-systems
PLANNED=$two_systems set_outcome two-systems feature-ssh failed 1 "$LOGS/eval-error.log" aarch64-linux
PLANNED=$two_systems report two-systems
expect_line two-systems '| `feature-ssh` (x86_64-linux) | ssh works in a VM | passed |'
expect_line two-systems '| `feature-ssh` (aarch64-linux) | ssh works in a VM | failed |'
expect_line two-systems '#### `feature-ssh` (aarch64-linux)'
expect_first_line two-systems "**1 check failed: no machine is affected, but CI stays red until it is fixed.**"

pass_all long
{
  printf '### Closure diff (x86_64-linux)\n\n#### example\n```\n'
  seq -f "[U.] package-%g: 1.0 -> 2.0" 20000
  printf '```\n'
} >long/closure-x86_64-linux.md
report long --comment
[ "$(wc -c <long.md)" -le 65536 ] || fail "long: the comment has $(wc -c <long.md) bytes, more than 65536"
expect_first_line long "**Everything works: every machine builds, boots and passed its tests.**"
[ "$(tail -n 1 long.md)" = "The report is too long for a comment: read it in full in the [run summary](https://github.com/owner/repo/actions/runs/42)." ] || fail "long: no pointer to the run summary: $(tail -n 3 long.md)"
[ $(($(grep -c '^```' long.md) % 2)) -eq 0 ] || fail "long: a code block is left open"
[ "$(grep -c '^<details' long.md)" -eq "$(grep -c '^</details>' long.md)" ] || fail "long: a details block is left open"
report long
grep -qx "\[U.\] package-20000: 1.0 -> 2.0" long.md || fail "long: the run summary is cut below its limit"

echo "ci-report: all assertions passed"
