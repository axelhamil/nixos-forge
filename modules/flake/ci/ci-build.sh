usage="usage: ci-build <system> <check> <out-dir> [--retry] [--base-input <input> --base <flake-ref>]
  RETRY=true stands for --retry; BASE_INPUT with BASE_SHA for --base-input \$BASE_INPUT --base github:\$GITHUB_REPOSITORY/\$BASE_SHA"

if [ $# -lt 3 ]; then
  echo "$usage" >&2
  exit 2
fi

system=$1
check=$2
out=$3
shift 3

allowed=1
base=""
base_input=""
while [ $# -gt 0 ]; do
  case "$1" in
  --retry) allowed=2 ;;
  --base | --base-input)
    [ $# -ge 2 ] || {
      echo "$usage" >&2
      exit 2
    }
    if [ "$1" = --base ]; then
      base=$2
    else
      base_input=$2
    fi
    shift
    ;;
  *)
    echo "$usage" >&2
    exit 2
    ;;
  esac
  shift
done

if [ "${RETRY:-}" = true ]; then
  allowed=2
fi

if [ -z "$base_input" ] && [ -n "${BASE_INPUT:-}" ]; then
  base_input=$BASE_INPUT
  base="github:${GITHUB_REPOSITORY:-}/${BASE_SHA:-}"
fi

if [ "${base:+set}" != "${base_input:+set}" ]; then
  echo "$usage" >&2
  exit 2
fi

override=()
case "$base" in
"") ;;
*/0000000000000000000000000000000000000000)
  echo "$check has no base revision ($base), it runs against its locked $base_input"
  ;;
*)
  echo "$check runs against $base as $base_input"
  override=(--override-input "$base_input" "$base")
  ;;
esac

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
touch "$work/excerpt"
echo null >"$work/boot-time"
echo '[]' >"$work/screenshots"
mkdir -p "$out"

write_result() {
  jq -n \
    --arg check "$check" \
    --arg system "$system" \
    --arg outcome "$outcome" \
    --argjson attempts "$attempts" \
    --rawfile excerpt "$work/excerpt" \
    --slurpfile bootTime "$work/boot-time" \
    --slurpfile screenshots "$work/screenshots" \
    '{ check: $check, system: $system, outcome: $outcome, attempts: $attempts, excerpt: ($excerpt | split("\n") | map(select(. != ""))), bootTime: $bootTime[0], screenshots: $screenshots[0] }' \
    >"$out/$system-$check.json"
}

keep_evidence() {
  local path
  path=$(head -n 1 "$work/out-paths") || return 1
  [ -n "$path" ] && [ -d "$path" ] || return 0

  if [ -f "$path/boot-time.json" ]; then
    jq -c '{ total, userspace }' "$path/boot-time.json" >"$work/boot-time.new" || return 1
    mv "$work/boot-time.new" "$work/boot-time" || return 1
  fi

  if [ -f "$path/perf.json" ]; then
    mkdir -p "$out/perf" || return 1
    install -m 644 "$path/perf.json" "$out/perf/$system.json" || return 1
  fi

  local screenshots=()
  for screenshot in "$path"/*.png; do
    [ -f "$screenshot" ] && screenshots+=("$screenshot")
  done
  [ "${#screenshots[@]}" -gt 0 ] || return 0

  mkdir -p "$out/screenshots" || return 1
  for screenshot in "${screenshots[@]}"; do
    install -m 644 "$screenshot" "$out/screenshots/$system-$check-$(basename "$screenshot")" || return 1
  done
  printf '%s\n' "${screenshots[@]##*/}" | jq -R . | jq -s -c . >"$work/screenshots.new" || return 1
  mv "$work/screenshots.new" "$work/screenshots"
}

outcome=failed
attempts=0
while [ "$attempts" -lt "$allowed" ]; do
  attempts=$((attempts + 1))

  if nix build --no-link --print-build-logs --print-out-paths "${override[@]}" ".#checks.$system.$check" 2>&1 >"$work/out-paths" | tee "$work/log"; then
    outcome=passed
    [ "$attempts" -eq 1 ] || outcome=unstable
    if ! keep_evidence; then
      echo "ci-build: the boot time, measures or screenshots of $check could not be kept, the check still passed" >&2
      echo null >"$work/boot-time"
      echo '[]' >"$work/screenshots"
    fi
    break
  fi

  awk -f "$EXCERPT" "$work/log" >"$work/excerpt"
  write_result
done

write_result

[ "$outcome" != failed ]
