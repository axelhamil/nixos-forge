usage="usage: ci-report [--comment]"

limit=1048576
footer="The report is too long for the run summary: the end is cut, the build logs hold every detail."

case "${1:-}" in
"") ;;
--comment)
  limit=65536
  footer="The report is too long for a comment: read it in full in the [run summary](${RUN_URL:-})."
  ;;
*)
  echo "$usage" >&2
  exit 2
  ;;
esac

results=${RESULTS_DIR:-results}

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

as_json() {
  local value
  value=$(jq -c "if type == \"$3\" then . else $2 end" <<<"${1:-null}" 2>/dev/null || true)
  echo "${value:-$2}"
}

shopt -s nullglob

for result in "$results"/*.json; do
  jq -c . "$result"
done | jq -s . >"$work/results.json"

for closure in "$results"/closure-*.md; do
  cat "$closure"
  echo
done >"$work/closures.md"

for perf in "$results"/perf/*.json; do
  jq -c --arg system "$(basename "$perf" .json)" 'objects | { ($system): . }' "$perf" 2>/dev/null || true
done | jq -s 'add // {}' >"$work/perf.json"

jq -n -r \
  --argjson planned "$(as_json "${PLANNED:-}" null array)" \
  --argjson skipped "$(as_json "${SKIPPED:-}" '[]' array)" \
  --argjson hosts "$(as_json "${HOST_CHECKS:-}" '{}' object)" \
  --argjson descriptions "$(as_json "${DESCRIPTIONS:-}" '{}' object)" \
  --argjson baseBoot "$(as_json "$(cat "$results/base/boot-time.json" 2>/dev/null || true)" '{}' object)" \
  --slurpfile results "$work/results.json" \
  --slurpfile perf "$work/perf.json" \
  --rawfile causes "$CAUSES" \
  --arg cache "${CACHE_OUTCOME:-}" \
  --arg lint "${LINT_RESULT:-}" \
  --arg plan "${PLAN_RESULT:-}" \
  --arg run "${RUN_URL:-}" \
  --from-file "$REPORT" >"$work/report.md"

if [ -s "$work/closures.md" ]; then
  {
    echo "<details>"
    echo "<summary>Package changes per machine</summary>"
    echo
    cat "$work/closures.md"
    echo "</details>"
  } >>"$work/report.md"
fi

LC_ALL=C awk -v limit="$limit" -v footer="$footer" -f "$TRUNCATE" "$work/report.md"
