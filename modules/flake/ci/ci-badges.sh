usage="usage: ci-badges checks <out-dir>
       ci-badges reproducibility <summary.md> <out-dir>
       ci-badges publish <dir>"

die() {
  echo "$usage" >&2
  exit 2
}

as_json() {
  local value
  value=$(jq -c "if type == \"$3\" then . else $2 end" <<<"${1:-null}" 2>/dev/null || true)
  echo "${value:-$2}"
}

badge() {
  jq -n --arg label "$2" --arg message "$3" --arg color "$4" \
    '{ schemaVersion: 1, label: $label, message: $message, color: $color }' >"$1"
}

checks() {
  local out=$1 results=${RESULTS_DIR:-results} work
  work=$(mktemp -d)
  mkdir -p "$out"

  shopt -s nullglob
  for result in "$results"/*.json; do
    jq -c . "$result" 2>/dev/null || true
  done | jq -s . >"$work/results.json"

  jq -n \
    --argjson planned "$(as_json "${PLANNED:-}" null array)" \
    --argjson skipped "$(as_json "${SKIPPED:-}" '[]' array)" \
    --argjson hosts "$(as_json "${HOST_CHECKS:-}" '{}' object)" \
    --argjson currentBoot "$(as_json "$(cat "$results/current/boot-time.json" 2>/dev/null || true)" '{}' object)" \
    --arg lint "${LINT_RESULT:-}" \
    --arg host "${BADGE_HOST:-example}" \
    --slurpfile results "$work/results.json" '
    def badge($label; $message; $color): { schemaVersion: 1, label: $label, message: $message, color: $color };

    def seconds: "\((. * 10 | round) / 10 | tostring | if test("[.]") then . else . + ".0" end) s";

    ($results[0] | map({ key: (.system + "." + .check), value: . }) | from_entries) as $byCheck
    | if $planned == null then
        { "checks.json": badge("checks"; "flake does not evaluate"; "red") }
      else
        ( [ $planned[] | . as $p | { check, system, vm: (.retry == true), ok: (($byCheck[$p.system + "." + $p.check].outcome // "") | IN("passed", "unstable")) } ]
          + [ $skipped[] | { check, system, vm: (.retry == true), ok: (.reason == "cached") } ]
        ) as $jobs
        | ($jobs + [ { check: "treefmt", ok: ($lint == "success") } ]) as $all
        | ($all | length) as $total
        | ($all | map(select(.ok)) | length) as $passing
        | ([ $hosts | to_entries[] | .key as $name | select(.value | index($name + "-boots")) | $name ] | sort) as $booting
        | (if ($booting | index($host)) then $host else $booting[0] end) as $bootHost
        | (if $bootHost == null then null
           else ([ ($results[0][] | select(.check == $bootHost + "-boots") | .bootTime.total), $currentBoot[$bootHost].total? ] | map(numbers) | first)
           end) as $boot
        | ([ $hosts | to_entries[] | .key as $name | .value as $checks
             | select(any($checks[]; . == $name + "-toplevel" or . == $name + "-sd-image"))
             | { name: $name, built: all($jobs[] | select(.check == $name + "-toplevel" or .check == $name + "-sd-image"); .ok) } ]) as $machines
        | ($machines | map(select(.built)) | length) as $built
        | {
            "checks.json": (if $passing == $total then badge("checks"; "\($total)/\($total) passing"; "brightgreen")
                            else badge("checks"; "\($total - $passing)/\($total) failing"; "red") end),
            "vm-tests.json": badge("vm tests"; "\($jobs | map(select(.vm)) | length)"; "5277C3"),
            "hosts.json": (if $built == ($machines | length) then badge("hosts"; "\($built) built"; "5277C3")
                           else badge("hosts"; "\($built)/\($machines | length) built"; "red") end)
          }
          + (if $boot == null then {} else { "boot-time.json": badge("boot to multi-user"; ($boot | seconds); "5277C3") } end)
      end
  ' >"$work/badges.json"

  jq -r 'keys[]' "$work/badges.json" | while read -r name; do
    jq --arg name "$name" '.[$name]' "$work/badges.json" >"$out/$name"
  done
  rm -rf "$work"
  ls "$out"
}

reproducibility() {
  local summary=$1 out=$2 differ
  mkdir -p "$out"
  [ -f "$summary" ] || {
    echo "ci-badges: $summary does not exist" >&2
    exit 1
  }

  if grep -qE '^- `[^`]+`: (the system did not build|the rebuild did not finish|`)' "$summary"; then
    badge "$out/reproducibility.json" reproducibility "not finished" lightgrey
  elif ! grep -qE '^- `[^`]+`: the rebuild' "$summary"; then
    badge "$out/reproducibility.json" reproducibility "no host" lightgrey
  else
    differ=$(grep -cE '^- `[^`]+`: the rebuild differs' "$summary" || true)
    case "$differ" in
    0) badge "$out/reproducibility.json" reproducibility reproducible brightgreen ;;
    1) badge "$out/reproducibility.json" reproducibility "1 path differs" orange ;;
    *) badge "$out/reproducibility.json" reproducibility "$differ paths differ" orange ;;
    esac
  fi
  cat "$out/reproducibility.json"
}

publish() {
  local dir=$1 attempt
  [ -d "$dir" ] || {
    echo "ci-badges: $dir does not exist" >&2
    exit 1
  }
  dir=$(realpath "$dir")
  work=$(mktemp -d)
  trap 'git worktree remove --force "$work/badges" 2>/dev/null || true; rm -rf "$work"' EXIT

  for attempt in 1 2 3; do
    git worktree remove --force "$work/badges" 2>/dev/null || true
    if git fetch -q --depth 1 origin badges 2>/dev/null; then
      git worktree add -q --detach "$work/badges" FETCH_HEAD
    else
      git worktree add -q --detach "$work/badges" HEAD
      git -C "$work/badges" checkout -q --orphan badges
      git -C "$work/badges" rm -q -r -f .
    fi

    cp "$dir"/*.json "$work/badges/"
    git -C "$work/badges" add -A
    if git -C "$work/badges" diff --cached --quiet; then
      echo "ci-badges: the badges did not change"
      return 0
    fi

    git -C "$work/badges" \
      -c user.name="github-actions[bot]" \
      -c user.email="41898282+github-actions[bot]@users.noreply.github.com" \
      commit -q -m "chore: update the badges"
    if git -C "$work/badges" push -q origin HEAD:refs/heads/badges; then
      echo "ci-badges: published $(cd "$dir" && echo *.json)"
      return 0
    fi
    echo "ci-badges: push $attempt was rejected, retrying on the new badges branch" >&2
  done

  echo "ci-badges: the badges could not be published" >&2
  exit 1
}

case "${1:-}" in
checks)
  [ $# -eq 2 ] || die
  checks "$2"
  ;;
reproducibility)
  [ $# -eq 3 ] || die
  reproducibility "$2" "$3"
  ;;
publish)
  [ $# -eq 2 ] || die
  publish "$2"
  ;;
-h | --help)
  echo "$usage"
  ;;
*) die ;;
esac
