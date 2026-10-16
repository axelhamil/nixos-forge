event=$1

kernels=$(nix eval --json .#ci.kernels)
jobs=$(nix eval --json .#ci.matrix)
closures=$(nix eval --json .#ci.closureMatrix)

evaluated=$(nix-eval-jobs --flake .#checks --force-recurse --check-cache-status --workers "$(nproc)" --max-memory-size 3072 | jq -s .)

jq -r 'map(select(.error)) | if length > 0 then (.[] | "\(.attr) does not evaluate: \(.error)\n") | halt_error(1) else empty end' <<<"$evaluated"

status=$(jq 'map({ key: .attr, value: { cached: .isCached, needs: (.neededBuilds // []) } }) | from_entries' <<<"$evaluated")

plan=$(
  jq -n -c \
    --arg event "$event" \
    --argjson kernels "$kernels" \
    --argjson jobs "$jobs" \
    --argjson closures "$closures" \
    --argjson status "$status" '
    def compiles($system; $needs): any($needs[]; IN(($kernels[$system] // [])[]));

    ($jobs | map(
      . as $job
      | ($status[$job.system + "." + $job.check] // error("\($job.system).\($job.check) is missing from the evaluation")) as $s
      | . + { cached: $s.cached, kernel: compiles($job.system; $s.needs) }
      | . + { build: ((.cached | not) and ($event == "push" or (.kernel | not))), timeout: (if .kernel then 240 else 60 end) }
    )) as $all
    | {
        checks: [$all[] | select(.build) | del(.cached, .kernel, .build)],
        skipped: [$all[] | select(.build | not) | { check, system, reason: (if .cached then "cached" else "compiles a kernel" end) }],
        closures: [$closures[] | . as $closure | . + { build: (any($all[]; .system == $closure.system and .kernel and (.cached | not)) | not) }]
      }
  '
)

if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
  {
    echo "### Plan"
    echo
    echo "$(jq '.checks | length' <<<"$plan") checks to build."
    jq -r '.skipped[] | "- `\(.check)` (\(.system)): \(.reason)"' <<<"$plan"
  } >>"$GITHUB_STEP_SUMMARY"
fi

echo "checks=$(jq -c .checks <<<"$plan")"
echo "closures=$(jq -c .closures <<<"$plan")"
