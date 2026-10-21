usage="usage: ci-flaky-plan"

case "${1:-}" in
-h | --help)
  echo "$usage"
  exit 0
  ;;
"") ;;
*)
  echo "$usage" >&2
  exit 2
  ;;
esac

runs=5

nothing() {
  echo "ci-flaky-plan: $1" >&2
  echo "checks=[]"
  echo "all=[]"
  exit 0
}

history=$(flaky-history)
tree=$(git rev-parse 'HEAD^{tree}')
if [ "$(jq -r '.tree // ""' <<<"$history")" = "$tree" ]; then
  nothing "this tree was already measured and every VM check passed all of its runs"
fi

matrix=$(nix eval --json .#ci.flakyMatrix)

jq -n -c --argjson matrix "$matrix" --argjson history "$history" --argjson runs "$runs" '
  def measured($job): ($history.checks[$job.drvPath].runs // []) as $r | ($r | length) >= $runs and all($r[]; .passed);
  "checks=\([$matrix[] | . as $job | select(measured($job) | not) | . + { runs: $runs }] | tojson)",
  "all=\($matrix | tojson)"
' | jq -r .
