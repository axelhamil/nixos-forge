fail() {
  echo "FAIL: $*" >&2
  exit 1
}

export STORE=$PWD/store
mkdir -p "$STORE"

mkdir -p "$STORE/same-system.check"
"$REPRODUCIBILITY" >summary.md || fail "a differing rebuild fails the run: $(cat summary.md)"
grep -qxF -- '- `same`: the rebuild is identical.' summary.md || fail "no identical line: $(cat summary.md)"
grep -qxF -- '- `differs`: the rebuild differs from the first build.' summary.md || fail "no differing line: $(cat summary.md)"
grep -q -- '-built at 1' summary.md || fail "the diffoscope summary misses the first build: $(cat summary.md)"
grep -q -- '+built at 2' summary.md || fail "the diffoscope summary misses the rebuild: $(cat summary.md)"
[ ! -e "$STORE/same-system.check" ] || fail "a .check left from an earlier run is kept"
[ "$(grep -c '^```$' summary.md)" -eq 2 ] || fail "the diffoscope block is not closed: $(cat summary.md)"

status=0
"$REPRODUCIBILITY" same broken >broken.md 2>broken.err || status=$?
[ "$status" -eq 1 ] || fail "a system that does not build exits with $status instead of 1"
grep -qxF -- '- `broken`: the system did not build, see the job log.' broken.md || fail "no build failure line: $(cat broken.md)"
grep -qxF -- '- `same`: the rebuild is identical.' broken.md || fail "a broken host stops the others: $(cat broken.md)"
grep -q "builder for 'broken' failed" broken.err || fail "the build error is not in the job log: $(cat broken.err)"

echo "reproducibility: all assertions passed"
