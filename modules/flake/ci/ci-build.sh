usage="usage: ci-build <system> <check> <out-dir> [--retry]"

if [ $# -lt 3 ] || [ $# -gt 4 ]; then
  echo "$usage" >&2
  exit 2
fi

system=$1
check=$2
out=$3

case "${4:-}" in
"") allowed=1 ;;
--retry) allowed=2 ;;
*)
  echo "$usage" >&2
  exit 2
  ;;
esac

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
touch "$work/excerpt"

outcome=failed
attempts=0
while [ "$attempts" -lt "$allowed" ]; do
  attempts=$((attempts + 1))

  if nix build --no-link --print-build-logs ".#checks.$system.$check" 2>&1 | tee "$work/log"; then
    outcome=passed
    [ "$attempts" -eq 1 ] || outcome=unstable
    break
  fi

  awk -f "$EXCERPT" "$work/log" >"$work/excerpt"
done

mkdir -p "$out"
jq -n \
  --arg check "$check" \
  --arg system "$system" \
  --arg outcome "$outcome" \
  --argjson attempts "$attempts" \
  --rawfile excerpt "$work/excerpt" \
  '{ check: $check, system: $system, outcome: $outcome, attempts: $attempts, excerpt: ($excerpt | split("\n") | map(select(. != ""))) }' \
  >"$out/$check.json"

[ "$outcome" != failed ]
