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
expect_excerpt substitute.log "^error: some substitutes for the outputs of derivation .* failed (usually happens due to networking issues)"

seq -f "machine # line %g of a healthy boot" 5000 >quiet.log
[ "$(awk -f "$EXCERPT" quiet.log | wc -l)" -eq 0 ] || fail "a log without failure yields an excerpt"

export NIX_CALLS

NIX_CALLS=$PWD/timeout-calls
status=0
"$TIMEOUT_CI_BUILD" x86_64-linux example-boots results >timeout.out || status=$?
[ "$status" -eq 1 ] || fail "timeout: a vm test failing once exits with $status instead of 1"
result=results/x86_64-linux-example-boots.json
jq -e '.check == "example-boots" and .system == "x86_64-linux"' "$result" >/dev/null || fail "timeout: wrong check or system: $(cat "$result")"
jq -e '.outcome == "failed"' "$result" >/dev/null || fail "timeout: a first failure is not kept as failed: $(cat "$result")"
jq -e '.excerpt | any(test("^RequestedAssertionFailed: action timed out"))' "$result" >/dev/null || fail "timeout: excerpt misses the failure: $(cat "$result")"
[ "$(wc -l <"$NIX_CALLS")" -eq 1 ] || fail "timeout: a failing check was built $(wc -l <"$NIX_CALLS") times instead of 1"
grep -qx "build --no-link --print-build-logs --print-out-paths .#checks.x86_64-linux.example-boots" "$NIX_CALLS" || fail "timeout: unexpected nix call: $(cat "$NIX_CALLS")"
grep -q "action timed out" timeout.out || fail "timeout: the full log is not on stdout"

NIX_CALLS=$PWD/broken-calls
status=0
"$BROKEN_CI_BUILD" x86_64-linux example-toplevel results >broken.out || status=$?
[ "$status" -eq 1 ] || fail "a failing check exits with $status instead of 1"
result=results/x86_64-linux-example-toplevel.json
jq -e '.outcome == "failed"' "$result" >/dev/null || fail "broken: not failed: $(cat "$result")"
jq -e '.bootTime == null and .screenshots == []' "$result" >/dev/null || fail "broken: a failed build has evidence: $(cat "$result")"
jq -e '.excerpt == ["error: undefined variable '"'"'foo'"'"'"]' "$result" >/dev/null || fail "broken: wrong excerpt: $(cat "$result")"

NIX_CALLS=$PWD/evidence-calls
"$EVIDENCE_CI_BUILD" x86_64-linux app-hyprland evidence >evidence.out || fail "evidence: a passing check exits non-zero"
result=evidence/x86_64-linux-app-hyprland.json
jq -e '.bootTime == { total: 7.04, userspace: 3.0 }' "$result" >/dev/null || fail "evidence: the boot time of the check output is not kept: $(cat "$result")"
jq -e '.screenshots == ["desktop.png"]' "$result" >/dev/null || fail "evidence: the screenshot is not listed: $(cat "$result")"
[ "$(cat evidence/screenshots/x86_64-linux-app-hyprland-desktop.png)" = png ] || fail "evidence: the screenshot is not copied for the artifact"
jq -e '.rounds == 3' evidence/perf/x86_64-linux.json >/dev/null || fail "evidence: the measures of the check output are not kept for the artifact"

NIX_CALLS=$PWD/unreadable-calls
"$UNREADABLE_EVIDENCE_CI_BUILD" x86_64-linux example-boots unreadable >unreadable.out 2>unreadable.err || fail "unreadable: evidence that cannot be read turns a passing check red"
result=unreadable/x86_64-linux-example-boots.json
jq -e '.outcome == "passed" and .bootTime == null and .screenshots == []' "$result" >/dev/null || fail "unreadable: wrong result: $(cat "$result")"
grep -q "could not be kept, the check still passed" unreadable.err || fail "unreadable: the lost evidence is not logged: $(cat unreadable.err)"

NIX_CALLS=$PWD/base-calls
"$BASE_CI_BUILD" x86_64-linux feature-migration results --base-input migration-base --base github:owner/repo/0123abc >base.out || fail "base: a passing check with a base exits non-zero"
grep -qx "build --no-link --print-build-logs --print-out-paths --override-input migration-base github:owner/repo/0123abc .#checks.x86_64-linux.feature-migration" "$NIX_CALLS" || fail "base: the check does not run against the base: $(cat "$NIX_CALLS")"

NIX_CALLS=$PWD/no-base-calls
"$BASE_CI_BUILD" x86_64-linux feature-migration results --base-input migration-base --base github:owner/repo/0000000000000000000000000000000000000000 >base.out || fail "base: a check whose push has no previous commit exits non-zero"
grep -qx "build --no-link --print-build-logs --print-out-paths .#checks.x86_64-linux.feature-migration" "$NIX_CALLS" || fail "base: an all-zero base revision is not left to the locked input: $(cat "$NIX_CALLS")"

NIX_CALLS=$PWD/env-base-calls
BASE_INPUT=migration-base BASE_SHA=0123abc GITHUB_REPOSITORY=owner/repo "$BASE_CI_BUILD" x86_64-linux feature-migration results >base.out || fail "base: a check with BASE_INPUT exits non-zero"
grep -qx "build --no-link --print-build-logs --print-out-paths --override-input migration-base github:owner/repo/0123abc .#checks.x86_64-linux.feature-migration" "$NIX_CALLS" || fail "base: BASE_INPUT and BASE_SHA do not run the check against the base: $(cat "$NIX_CALLS")"

NIX_CALLS=$PWD/env-empty-base-calls
BASE_INPUT='' BASE_SHA=0123abc GITHUB_REPOSITORY=owner/repo "$BASE_CI_BUILD" x86_64-linux feature-migration results >base.out || fail "base: a check with an empty BASE_INPUT exits non-zero"
grep -qx "build --no-link --print-build-logs --print-out-paths .#checks.x86_64-linux.feature-migration" "$NIX_CALLS" || fail "base: an empty BASE_INPUT overrides an input: $(cat "$NIX_CALLS")"

if "$BASE_CI_BUILD" x86_64-linux feature-migration results --base github:owner/repo/0123abc >base.out 2>&1; then
  fail "base: a base without the input it overrides is accepted"
fi

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
    jq '{ check, system, outcome: "passed", excerpt: [] }' <<<"$job" >"$1/$(jq -r '.system + "-" + .check' <<<"$job").json"
  done
}

set_outcome() {
  local dir=$1 check=$2 outcome=$3 log=${4:-} system=${5:-x86_64-linux}
  local excerpt='[]'
  local file=$dir/$system-$check.json
  [ -z "$log" ] || excerpt=$(awk -f "$EXCERPT" "$log" | jq -R . | jq -s -c .)
  jq --arg outcome "$outcome" --argjson excerpt "$excerpt" \
    '.outcome = $outcome | .excerpt = $excerpt' "$file" >"$file.tmp"
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
expect_line all-green '| `example-disk` | it installs on an encrypted disk, wipes / at boot and takes a snapshot | passed |'
expect_line all-green '| `feature-ssh` | ssh works in a VM | passed |'
expect_line all-green '| `treefmt` | the code is formatted | passed |'
expect_line all-green "<summary>Module checks: 3 passed</summary>"
expect_line all-green "<summary>Package changes per machine</summary>"
expect_line all-green "new host"
if grep -q "What went wrong" all-green.md; then fail "all-green: a green run lists failures"; fi

pass_all one-failed
set_outcome one-failed example-server-boots failed "$LOGS/assertion.log"
report one-failed
expect_first_line one-failed "**1 check failed: example-server would not get this change.**"
expect_rows one-failed example "example-toplevel=passed example-boots=passed example-disk=passed example-wipe-home=passed"
expect_rows one-failed example-server "example-server-toplevel=passed example-server-boots=failed example-server-disk=passed example-server-installer=passed"
expect_line one-failed '#### `example-server-boots` (x86_64-linux)'
expect_line one-failed "- Cause: the system started but a test found it behaving wrong."
expect_line one-failed '- What to do: read the failing line, then reproduce with `nix build -L .#checks.x86_64-linux.example-server-boots`.'
expect_line one-failed "AssertionError: doctor misses a file written to /"
expect_line one-failed "[Open the full log](https://github.com/owner/repo/actions/runs/42)"

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
set_outcome module-failed feature-ssh failed "$LOGS/eval-error.log"
LINT_RESULT=failure report module-failed
expect_first_line module-failed "**2 checks failed: no machine is affected, but CI stays red until it is fixed.**"
expect_line module-failed "<details open>"
expect_line module-failed "- Cause: the configuration does not evaluate."
expect_line module-failed '- What to do: run `nix fmt`, then commit the result.'

pass_all timeout
set_outcome timeout example-boots failed "$LOGS/timeout.log"
report timeout
expect_line timeout "- Cause: a VM test hit its time limit."
expect_line timeout '- What to do: read the wait the log names before the timeout: the unit, file or text it waited for never came; reproduce with `nix build -L .#checks.x86_64-linux.example-boots`.'

printf 'machine # [   12.0] systemd[1]: Started backup.service.\nRequestedAssertionFailed: command `test -e /persist/state` failed (exit code 1)\n' >assertion-only.log
pass_all assertion-only
set_outcome assertion-only example-boots failed "$PWD/assertion-only.log"
report assertion-only
expect_line assertion-only "- Cause: the system started but a test found it behaving wrong."

printf 'machine # [    8.0] dhcpcd[612]: eth0: DHCPv6 solicit timed out\nAssertionError: the greeter never showed\n' >guest-noise.log
pass_all guest-noise
set_outcome guest-noise example-boots failed "$PWD/guest-noise.log"
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
set_outcome fence feature-ssh failed "$PWD/fence.log"
report fence
expect_line fence "\`\`\`\`markdown"
expect_line fence "\`\`\`\`\`"
[ "$(grep -c '^`````$' fence.md)" -eq 2 ] || fail "fence: the excerpt fence is not longer than the backticks it holds: $(cat fence.md)"

pass_all escape
for name in 'system-a|b' 'app-x`y'; do
  jq -n --arg check "$name" '{ check: $check, system: "x86_64-linux", outcome: "passed", excerpt: [] }' >"escape/x86_64-linux-$name.json"
done
PLANNED=$(jq -c '. + [{ check: "system-a|b", system: "x86_64-linux" }, { check: "app-x`y", system: "x86_64-linux" }]' <<<"$PLANNED") report escape
expect_line escape '| `system-a\|b` | its own test passes | passed |'
expect_line escape '| ``app-x`y`` | its own test passes | passed |'

pass_all compile
set_outcome compile example-toplevel failed "$LOGS/compile-error.log"
report compile
expect_line compile "- Cause: a package or test failed to build."
expect_line compile '- What to do: read the lines above `Cannot build` in the full log.'
expect_line compile "main.c:3:1: error: expected ';' before '}' token"

pass_all hash
set_outcome hash example-toplevel failed "$LOGS/hash-mismatch.log"
report hash
expect_line hash "- Cause: a download changed upstream."

pass_all substitute
set_outcome substitute example-toplevel failed "$LOGS/substitute.log"
report substitute
expect_line substitute "- Cause: the binary cache could not be reached."
expect_line substitute "- What to do: rerun the job; nothing in the change is at fault."

grep -h "^warning:" "$LOGS/substitute.log" >download-warnings.log
echo "AssertionError: the greeter never showed" >>download-warnings.log
pass_all download-warnings
set_outcome download-warnings example-boots failed "$PWD/download-warnings.log"
report download-warnings
expect_line download-warnings "- Cause: the system started but a test found it behaving wrong."

cat "$LOGS/substitute.log" "$LOGS/assertion.log" >substitute-and-assertion.log
pass_all substitute-and-assertion
set_outcome substitute-and-assertion example-boots failed "$PWD/substitute-and-assertion.log"
report substitute-and-assertion
expect_line substitute-and-assertion "- Cause: the system started but a test found it behaving wrong."

cat "$LOGS/substitute.log" "$LOGS/compile-error.log" >substitute-and-compile.log
pass_all substitute-and-compile
set_outcome substitute-and-compile example-toplevel failed "$PWD/substitute-and-compile.log"
report substitute-and-compile
expect_line substitute-and-compile "- Cause: a package or test failed to build."

printf 'error: writing to file: No space left on device\nerror: Cannot build '"'"'/nix/store/x-example.drv'"'"'.\n' >no-space.log
pass_all no-space
set_outcome no-space example-toplevel failed "$PWD/no-space.log"
report no-space
expect_line no-space "- Cause: the CI runner ran out of disk."

two_systems=$(jq -c '. + [{ check: "feature-ssh", system: "aarch64-linux" }]' <<<"$PLANNED")
PLANNED=$two_systems pass_all two-systems
PLANNED=$two_systems set_outcome two-systems feature-ssh failed "$LOGS/eval-error.log" aarch64-linux
PLANNED=$two_systems report two-systems
expect_line two-systems '| `feature-ssh` (x86_64-linux) | ssh works in a VM | passed |'
expect_line two-systems '| `feature-ssh` (aarch64-linux) | ssh works in a VM | failed |'
expect_line two-systems '#### `feature-ssh` (aarch64-linux)'
expect_first_line two-systems "**1 check failed: no machine is affected, but CI stays red until it is fixed.**"

pass_all boot-time
jq '.bootTime = { total: 7.04, userspace: 3.0 }' boot-time/x86_64-linux-example-boots.json >boot-time/tmp.json
mv boot-time/tmp.json boot-time/x86_64-linux-example-boots.json
jq '.screenshots = ["desktop.png"]' boot-time/x86_64-linux-feature-ssh.json >boot-time/tmp.json
mv boot-time/tmp.json boot-time/x86_64-linux-feature-ssh.json
mkdir -p boot-time/base
echo '{ "example": { "total": 6.8, "userspace": null }, "example-server": { "total": 9, "userspace": 4.25 } }' >boot-time/base/boot-time.json
report boot-time
expect_first_line boot-time "**Everything works: every machine builds, boots and passed its tests.**"
expect_line boot-time "### Boot time"
expect_line boot-time "| example | 7.0 s | 6.8 s | 3.0 s | n/a |"
expect_line boot-time "| example-server | n/a | 9.0 s | n/a | 4.3 s |"
if grep -q "^| example-pi |" boot-time.md; then fail "boot-time: a machine without a boot test gets a boot time"; fi
expect_line boot-time '- `feature-ssh` (x86_64-linux): `desktop.png` in the artifact `screenshot-x86_64-linux-feature-ssh` of [this run](https://github.com/owner/repo/actions/runs/42#artifacts).'
expect_rows boot-time example "example-toplevel=passed example-boots=passed example-disk=passed example-wipe-home=passed"
if grep -q "### Boot time\|### Screenshots" all-green.md; then fail "all-green: a run without boot time or screenshot shows their section"; fi

pass_all no-base
jq '.bootTime = { total: 7.04, userspace: 3.0 }' no-base/x86_64-linux-example-boots.json >no-base/tmp.json
mv no-base/tmp.json no-base/x86_64-linux-example-boots.json
mkdir -p no-base/base
echo 'not json' >no-base/base/boot-time.json
report no-base
expect_line no-base "| example | 7.0 s | n/a | 3.0 s | n/a |"
expect_line no-base "| example-server | n/a | n/a | n/a | n/a |"

perf_side() {
  jq -n -c --argjson total "$1" --argjson userspace "$2" --argjson memory "$3" --argjson closure "$4" --argjson services "$5" \
    '{ total: $total, userspace: $userspace, memory: $memory, services: $services, cpu: 7500, memoryBandwidth: 50000, randomRead: 50000 } as $boot
    | { min: $boot, median: $boot, closure: $closure, samples: [] }'
}

perf_pair() {
  jq -n -c --argjson base "$(perf_side "${@:1:5}")" --argjson current "$(perf_side "${@:6:5}")" '{ base: $base, current: $current }'
}

perf_run() {
  local dir=$1
  pass_all "$dir"
  mkdir -p "$dir/perf"
  jq -n --argjson example "$2" --argjson server "$3" \
    '{ rounds: 5, hosts: { example: $example, "example-server": $server } }' >"$dir/perf/x86_64-linux.json"
}

steady=$(perf_pair 7.0 3.0 400 2147483648 20 7.0 3.0 400 2147483648 20)
noisy_median=$(jq -c '.current.median.total = 9.9' <<<"$steady")

perf_run perf-steady "$steady" "$noisy_median"
report perf-steady
expect_first_line perf-steady "**Everything works: every machine builds, boots and passed its tests.**"
expect_line perf-steady "### Performance against the base"
expect_line perf-steady "No metric crossed its threshold."
expect_line perf-steady "#### example-server"
expect_line perf-steady "| Boot, total (best) | 7.0 s | 7.0 s | 0.0 % |"
expect_line perf-steady "| Idle RAM (best) | 400.0 MiB | 400.0 MiB | 0.0 % |"
expect_line perf-steady "| Closure | 2048.0 MiB | 2048.0 MiB | 0.0 % |"
expect_line perf-steady "| Running services (median) | 20 | 20 | 0.0 % |"

perf_run perf-regressed "$(perf_pair 7.0 3.0 400 2147483648 20 7.8 3.2 448 2147483648 21)" \
  "$(perf_pair 7.0 3.0 400 2147483648 20 6.0 3.0 400 2361393152 19)"
report perf-regressed
expect_first_line perf-regressed "**Everything works: every machine builds, boots and passed its tests.**"
expect_line perf-regressed "4 metrics crossed their threshold:"
expect_line perf-regressed '- `example`: Boot, total +11.4 %, +0.8 s (threshold: +10 % and +0.5 s).'
expect_line perf-regressed '- `example`: Idle RAM +12.0 %, +48.0 MiB (threshold: +10 % and +32.0 MiB).'
expect_line perf-regressed '- `example`: Running services +5.0 %, +1 (threshold: any increase).'
expect_line perf-regressed '- `example-server`: Closure +10.0 %, +204.0 MiB (threshold: +5 % and +50.0 MiB).'
if grep -q "Boot, userspace +" perf-regressed.md; then fail "perf-regressed: a change under its threshold is flagged"; fi
expect_line perf-regressed "| Boot, total (best) | 7.0 s | 6.0 s | -14.3 % |"
if grep -q "Fix what is listed below" perf-regressed.md; then fail "perf-regressed: a regression turns the report red"; fi

perf_run perf-benchmarks "$(jq -c '.current.median += { cpu: 7000, memoryBandwidth: 60000, randomRead: 46000 }' <<<"$steady")" \
  "$(jq -c '.current.median += { cpu: 7600, memoryBandwidth: 45000, randomRead: 42000 }' <<<"$steady")"
report perf-benchmarks
expect_line perf-benchmarks "3 metrics crossed their threshold:"
expect_line perf-benchmarks '- `example`: CPU, sysbench -6.7 %, -500.0 events/s (threshold: -5 % and -100.0 events/s).'
expect_line perf-benchmarks '- `example-server`: Disk, 4k random read -16.0 %, -8000 IOPS (threshold: -10 % and -2000 IOPS).'
expect_line perf-benchmarks '- `example-server`: Memory bandwidth, sysbench -10.0 %, -5000.0 MiB/s (threshold: -5 % and -1000.0 MiB/s).'
if grep -q "Memory bandwidth, sysbench +\|example\`: Disk, 4k random read\|example-server\`: CPU" perf-benchmarks.md; then fail "perf-benchmarks: a benchmark that rose or dropped under its threshold is flagged"; fi
expect_line perf-benchmarks "| CPU, sysbench (median) | 7500.0 events/s | 7000.0 events/s | -6.7 % |"
expect_line perf-benchmarks "| Memory bandwidth, sysbench (median) | 50000.0 MiB/s | 60000.0 MiB/s | +20.0 % |"
expect_line perf-benchmarks "| Disk, 4k random read (median) | 50000 IOPS | 46000 IOPS | -8.0 % |"
expect_line perf-benchmarks "| Disk, 4k random read (median) | 50000 IOPS | 42000 IOPS | -16.0 % |"
if grep -q "Fix what is listed below" perf-benchmarks.md; then fail "perf-benchmarks: a benchmark drop turns the report red"; fi

perf_run perf-benchmarks-small "$(jq -c '.base.median.cpu = 1000 | .current.median.cpu = 940' <<<"$steady")" "$steady"
report perf-benchmarks-small
expect_line perf-benchmarks-small "No metric crossed its threshold."
expect_line perf-benchmarks-small "| CPU, sysbench (median) | 1000.0 events/s | 940.0 events/s | -6.0 % |"

perf_run perf-small "$(perf_pair 2.0 1.0 200 209715200 20 2.3 1.0 225 241172480 20)" "$(perf_pair 7.0 3.0 400 3155385672 20 7.0 3.0 400 3154346392 20)"
report perf-small
expect_line perf-small "No metric crossed its threshold."
expect_line perf-small "| Boot, total (best) | 2.0 s | 2.3 s | +15.0 % |"
expect_line perf-small "| Idle RAM (best) | 200.0 MiB | 225.0 MiB | +12.5 % |"
expect_line perf-small "| Closure | 200.0 MiB | 230.0 MiB | +15.0 % |"
expect_line perf-small "| Closure | 3009.2 MiB | 3008.2 MiB | 0.0 % |"

perf_run perf-partial "$(jq -n -c --argjson side "$(perf_side 7.0 3.0 400 2147483648 20)" '{ current: $side }')" '"not an object"'
report perf-partial
expect_line perf-partial "No metric crossed its threshold."
expect_line perf-partial "| Boot, total (best) | n/a | 7.0 s | n/a |"
if grep -q "^#### example-server" perf-partial.md; then fail "perf-partial: a machine without measures gets a table"; fi

perf_run perf-fork-host "$steady" "$steady"
jq '.skipped = { laptop: "absent", pi: "no-vm" }' perf-fork-host/perf/x86_64-linux.json >perf-fork-host/tmp.json
mv perf-fork-host/tmp.json perf-fork-host/perf/x86_64-linux.json
report perf-fork-host
expect_line perf-fork-host "#### example-server"
expect_line perf-fork-host "2 machines are not measured:"
expect_line perf-fork-host '- `laptop`: not on the base revision, so nothing to compare it with.'
expect_line perf-fork-host '- `pi`: it does not boot in a VM on the base revision.'

pass_all perf-none-on-base
mkdir -p perf-none-on-base/perf
echo '{ "rounds": 5, "hosts": {}, "skipped": { "laptop": "absent" } }' >perf-none-on-base/perf/x86_64-linux.json
report perf-none-on-base
expect_first_line perf-none-on-base "**Everything works: every machine builds, boots and passed its tests.**"
expect_line perf-none-on-base "### Performance against the base"
expect_line perf-none-on-base "1 machine is not measured:"
expect_line perf-none-on-base '- `laptop`: not on the base revision, so nothing to compare it with.'
if grep -q "No metric crossed its threshold." perf-none-on-base.md; then fail "perf-none-on-base: a run that measured nothing says no metric crossed its threshold"; fi

pass_all perf-unreadable
mkdir -p perf-unreadable/perf
echo 'not json' >perf-unreadable/perf/x86_64-linux.json
report perf-unreadable
if grep -q "### Performance" perf-unreadable.md all-green.md; then fail "a run without readable measures shows the performance section"; fi
expect_first_line perf-unreadable "**Everything works: every machine builds, boots and passed its tests.**"

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

pass_all green-note
report green-note
[ "$(sed -n 3p green-note.md)" = "Nothing to fix: this commit is ready for the machines." ] || fail "green-note: the note is not under the verdict: $(head -n 3 green-note.md)"
if grep -q "Fix what is listed below" green-note.md; then fail "green-note: a green report asks for a fix"; fi

pass_all red-note
set_outcome red-note example-server-boots failed "$LOGS/assertion.log"
report red-note --comment
[ "$(sed -n 3p red-note.md)" = "Fix what is listed below and push again: CI runs on every push." ] || fail "red-note: the note is not under the verdict: $(head -n 3 red-note.md)"
[ "$(grep -c "Fix what is listed below" red-note.md)" -eq 1 ] || fail "red-note: the note is repeated in the details"

PLANNED='' PLAN_RESULT=failure report red-note-no-plan
[ "$(sed -n 3p red-note-no-plan.md)" = "Fix what is listed below and push again: CI runs on every push." ] || fail "red-note-no-plan: the note is not under the verdict: $(head -n 3 red-note-no-plan.md)"

echo "ci-report: all assertions passed"
