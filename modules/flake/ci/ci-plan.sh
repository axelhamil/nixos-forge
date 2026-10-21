event=$1

kernels=$(nix eval --json .#ci.kernels)
jobs=$(nix eval --json .#ci.matrix)
closures=$(nix eval --json .#ci.closureMatrix)
perf=$(nix eval --json .#ci.perfMatrix)
host_checks=$(nix eval --json .#ci.hostChecks)
descriptions=$(nix eval --json .#ci.descriptions)

evaluated=$(nix-eval-jobs --flake .#checks --force-recurse --check-cache-status --workers "$(nproc)" --max-memory-size 3072 | jq -s .)

jq -r 'map(select(.error)) | if length > 0 then (.[] | "\(.attr) does not evaluate: \(.error)\n") | halt_error(1) else empty end' <<<"$evaluated"

status=$(
  jq --argjson kernels "$kernels" '
    map(
      (.attr | split(".")[0]) as $system
      | { key: .attr, value: { cached: .isCached, kernel: any((.neededBuilds // [])[]; IN(($kernels[$system] // [])[])) } }
    )
    | from_entries
  ' <<<"$evaluated"
)

plan=$(
  jq -n -c \
    --arg event "$event" \
    --argjson jobs "$jobs" \
    --argjson closures "$closures" \
    --argjson status "$status" '
    ($jobs | map(
      . as $job
      | ($status[$job.system + "." + $job.check] // error("\($job.system).\($job.check) is missing from the evaluation")) as $s
      | . + { cached: ($s.cached and $job.baseInput == null), kernel: $s.kernel }
      | . + { build: ((.cached | not) and ($event == "push" or (.kernel | not))), timeout: (if .kernel then 240 else 60 end) }
    )) as $all
    | {
        checks: [$all[] | select(.build) | del(.cached, .kernel, .build)],
        skipped: [$all[] | select(.build | not) | { check, system, vm, reason: (if .cached then "cached" else "compiles a kernel" end) }],
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
echo "skipped=$(jq -c .skipped <<<"$plan")"
echo "closures=$(jq -c .closures <<<"$plan")"
echo "perf=$(jq -c . <<<"$perf")"
echo "hostChecks=$(jq -c . <<<"$host_checks")"
echo "descriptions=$(jq -c . <<<"$descriptions")"
