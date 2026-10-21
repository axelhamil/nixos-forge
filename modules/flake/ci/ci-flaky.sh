usage="usage: ci-flaky <system> <check> <runs> <out-dir>"

case "${1:-}" in
-h | --help)
  echo "$usage"
  exit 0
  ;;
esac

if [ $# -ne 4 ] || ! [[ "$3" =~ ^[1-9][0-9]*$ ]]; then
  echo "$usage" >&2
  exit 2
fi

system=$1
check=$2
runs=$3
out=$4
installable=".#checks.$system.$check"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$out"
touch "$work/runs"

realised=false
echo "ci-flaky: realising $check once, not counted as a run"
if nix build --no-link --print-build-logs "$installable"; then
  realised=true
else
  echo "ci-flaky: $check did not realise, each run builds it until one passes" >&2
fi

only_differs() {
  grep -q 'may not be deterministic' "$work/log" && ! grep -qE 'error: (builder for|Cannot build)' "$work/log"
}

for run in $(seq "$runs"); do
  rebuild=()
  [ "$realised" = false ] || rebuild=(--rebuild)

  start=$SECONDS
  passed=false
  if nix build --no-link --print-build-logs "${rebuild[@]}" "$installable" 2>&1 | tee "$work/log"; then
    passed=true
  elif [ "$realised" = true ] && only_differs; then
    passed=true
  fi
  [ "$passed" = false ] || realised=true

  jq -n -c --argjson passed "$passed" --argjson seconds "$((SECONDS - start))" \
    '{ passed: $passed, seconds: $seconds }' >>"$work/runs"
  echo "ci-flaky: run $run of $runs of $check passed: $passed"
done

jq -s --arg check "$check" --arg system "$system" \
  '{ check: $check, system: $system, runs: . }' "$work/runs" >"$out/$system-$check.json"
