if [ $# -ne 1 ] || [ -z "$1" ]; then
  echo "usage: ci-pin-kernels <cache-name>" >&2
  exit 2
fi

cache=$1

if [ -z "${CACHIX_AUTH_TOKEN:-}" ]; then
  echo "CACHIX_AUTH_TOKEN is empty: set the CACHIX_AUTH_TOKEN secret to a token that can write to $cache" >&2
  exit 1
fi

pins=$(nix eval --json .#ci.kernelPins)
rows=$(jq -r '.[] | [.name, .path] | @tsv' <<<"$pins")
[ -n "$rows" ] || exit 0

failed=0
while IFS=$'\t' read -r name path; do
  if ! nix path-info --store "https://$cache.cachix.org" "$path" >/dev/null 2>&1; then
    echo "not in $cache, not pinned: $name $path"
    continue
  fi

  if cachix pin "$cache" "$name" "$path" --keep-revisions 3; then
    echo "pinned $name $path"
  else
    echo "failed to pin $name $path" >&2
    failed=1
  fi
done <<<"$rows"

exit "$failed"
