usage="usage: ci-promote <results-dir> <sha>"

if [ $# -ne 2 ]; then
  echo "$usage" >&2
  exit 2
fi

results=$1
sha=$(git rev-parse --verify "$2^{commit}")
short=$(git rev-parse --short "$sha")

for input in HOST_CHECKS PLANNED SKIPPED; do
  if ! jq -e 'type == (if $name == "HOST_CHECKS" then "object" else "array" end)' --arg name "$input" <<<"${!input:-null}" >/dev/null 2>&1; then
    echo "ci-promote: $input must hold the $input JSON of ci-plan" >&2
    exit 2
  fi
done

shopt -s nullglob

verdicts=$(
  for result in "$results"/*.json; do
    jq -c . "$result"
  done | jq -s -r \
    --argjson hosts "$HOST_CHECKS" \
    --argjson planned "$PLANNED" \
    --argjson skipped "$SKIPPED" '
    (map({ key: (.system + "." + .check), value: .outcome }) | from_entries) as $outcomes
    | ([ $planned[] | { check, status: ($outcomes[.system + "." + .check] // "did not finish") } ]
       + [ $skipped[] | { check, status: (if .reason == "cached" then "passed" else "skipped" end) } ]) as $statuses
    | $hosts | to_entries | sort_by(.key)[]
    | .key as $host
    | [ .value[] as $check
        | ([ $statuses[] | select(.check == $check) | .status ] | if . == [] then [ "did not finish" ] else . end)[]
        | select(IN("passed", "unstable") | not) as $status
        | "\($check) \($status)" ] as $kept
    | if $kept == [] then "\($host)\tpromote" else "\($host)\tkept: \($kept | unique | join(", "))" end
  '
)

git fetch -q origin '+refs/heads/deploy/*:refs/remotes/origin/deploy/*'

refused=0
while IFS=$'\t' read -r host verdict; do
  if [ "$verdict" != promote ]; then
    echo "deploy/$host $verdict"
    continue
  fi

  if git rev-parse -q --verify "refs/remotes/origin/deploy/$host" >/dev/null && git merge-base --is-ancestor "$sha" "refs/remotes/origin/deploy/$host"; then
    echo "deploy/$host already at or past $short"
    continue
  fi

  if output=$(git push --porcelain origin "$sha:refs/heads/deploy/$host" 2>&1); then
    echo "deploy/$host -> $short"
    continue
  fi

  refused=1
  reason=$(awk -F'\t' '/^!/ { print $3; exit }' <<<"$output")
  if grep -qE 'non-fast-forward|fetch first' <<<"$reason"; then
    echo "deploy/$host refused: not a fast-forward"
  else
    echo "deploy/$host refused: ${reason:-$(grep -v '^Done$' <<<"$output" | tail -n 1)}"
  fi
done <<<"$verdicts"

exit "$refused"
