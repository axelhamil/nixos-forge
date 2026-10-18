usage="usage: ci-boot-time <flake ref> <out-dir>"

if [ $# -ne 2 ]; then
  echo "$usage" >&2
  exit 2
fi

ref=$1
out=$2

mkdir -p "$out"

boot_tests=$(
  jq -n -r \
    --argjson hosts "$(nix eval --json "$ref#ci.hosts")" \
    --argjson checks "$(nix eval --json "$ref#ci.hostChecks")" \
    '$hosts | to_entries[] | .key as $system | .value[] as $host | select(($checks[$host] // []) | index($host + "-boots")) | "\($system) \($host)"'
)

while read -r system host; do
  [ -n "$host" ] || continue
  path=$(nix build --no-link --print-out-paths --max-jobs 0 "$ref#checks.$system.$host-boots" 2>/dev/null) || {
    echo "$host: not in the binary cache" >&2
    continue
  }
  if [ ! -f "$path/boot-time.json" ]; then
    echo "$host: the boot test recorded no boot time" >&2
    continue
  fi
  jq -c --arg host "$host" '{ ($host): { total, userspace } }' "$path/boot-time.json"
done <<<"$boot_tests" | jq -s 'add // {}' >"$out/boot-time.json"

cat "$out/boot-time.json"
