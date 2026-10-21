usage="usage: MATRIX=<all> PLANNED=<checks> ci-flaky-report <results-dir> <history-out-dir>"

case "${1:-}" in
-h | --help)
  echo "$usage"
  exit 0
  ;;
esac

if [ $# -ne 2 ] || [ -z "${MATRIX:-}" ]; then
  echo "$usage" >&2
  exit 2
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

shopt -s nullglob
results=("$1"/*.json)
if [ "${#results[@]}" -gt 0 ]; then
  jq -s . "${results[@]}" >"$work/results.json"
else
  echo '[]' >"$work/results.json"
fi

flaky-history >"$work/history.json"

jq -n \
  --argjson matrix "$MATRIX" \
  --argjson planned "${PLANNED:-[]}" \
  --arg tree "$(git rev-parse 'HEAD^{tree}')" \
  --slurpfile history "$work/history.json" \
  --slurpfile results "$work/results.json" \
  -f "$REPORT" >"$work/report.json"

mkdir -p "$2"
jq '.history' "$work/report.json" >"$2/flaky.json"
jq -r '.markdown[]' "$work/report.json"
jq -e '.ok' "$work/report.json" >/dev/null
