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
        | select(. != "passed") as $status
        | "\($check) \($status)" ] as $kept
    | if $kept == [] then "\($host)\tpromote" else "\($host)\tkept: \($kept | unique | join(", "))" end
  '
)

git fetch -q --prune origin '+refs/heads/deploy/*:refs/remotes/origin/deploy/*'

signing=()
if [ -n "${DEPLOY_SIGNING_KEY:-}" ]; then
  key_dir=$(mktemp -d)
  trap 'rm -rf "$key_dir"' EXIT
  (
    umask 077
    printf '%s\n' "$DEPLOY_SIGNING_KEY" >"$key_dir/key"
  )
  signing=(-c gpg.format=ssh -c user.signingKey="$key_dir/key")
  if public=$(ssh-keygen -y -P "" -f "$key_dir/key" 2>/dev/null </dev/null); then
    printf 'deploy namespaces="git" %s\n' "$public" >"$key_dir/allowed"
  else
    : >"$key_dir/allowed"
  fi
  if ! git config user.email >/dev/null; then
    signing+=(-c user.name=ci-promote -c user.email=ci-promote@localhost)
  fi
else
  echo "unsigned: DEPLOY_SIGNING_KEY is not set, so deploy branches point to main's own commits, which hosts with update.signers refuse"
fi

warn() {
  echo "$1"
  if [ -n "${GITHUB_ACTIONS:-}" ]; then
    echo "::warning::$1"
  fi
}

verified() {
  local host=$1 commit=$2
  git -c gpg.format=ssh -c gpg.ssh.allowedSignersFile="$key_dir/allowed" -c gpg.openpgp.program=false -c gpg.x509.program=false verify-commit "$commit" >/dev/null 2>&1 &&
    [[ $(git log -1 --format=%s "$commit") == "deploy $host: checks passed for "* ]]
}

carried() {
  local host=$1 commit=$2 parents
  if [[ $(git log -1 --format=%s "$commit") == "deploy $host: checks passed for "* ]]; then
    read -r -a parents < <(git rev-list --parents -n 1 "$commit")
    echo "${parents[-1]}"
  else
    echo "$commit"
  fi
}

promotion() {
  local host=$1 previous=$2 parents=()
  if [ -n "$previous" ] && ! git merge-base --is-ancestor "$previous" "$sha"; then
    parents+=(-p "$previous")
  fi
  parents+=(-p "$sha")
  git "${signing[@]}" commit-tree -S "${parents[@]}" -m "deploy $host: checks passed for $short" "$sha^{tree}"
}

refused=0

refuse() {
  refused=1
  echo "deploy/$1 refused: $2"
}

push_deploy() {
  local host=$1 target=$2 moved=$3 output reason
  shift 3
  if output=$(git push --porcelain "$@" origin "$target:refs/heads/deploy/$host" 2>&1); then
    echo "$moved"
    return
  fi

  reason=$(awk -F'\t' '/^!/ { print $3; exit }' <<<"$output")
  if grep -qE 'non-fast-forward|fetch first|stale info' <<<"$reason"; then
    refuse "$host" "not a fast-forward"
    return
  fi
  refuse "$host" "${reason:-$(grep -v '^Done$' <<<"$output" | tail -n 1)}"
}

promote_unsigned() {
  local host=$1 tip=$2
  if [ -n "$tip" ] && git merge-base --is-ancestor "$sha" "$tip"; then
    echo "deploy/$host already at or past $short"
    return
  fi
  push_deploy "$host" "$sha" "deploy/$host -> $short"
}

promote_signed() {
  local host=$1 tip=$2 previous="" lease=() target
  if [ -z "$tip" ] || verified "$host" "$tip"; then
    previous=$tip
  else
    warn "deploy/$host warning: its tip $(git rev-parse --short "$tip") is not a promotion signed by DEPLOY_SIGNING_KEY, so the promotion replaces it instead of building on it"
    lease=(--force-with-lease="refs/heads/deploy/$host:$tip")
  fi
  if [ -n "$previous" ] && git merge-base --is-ancestor "$sha" "$(carried "$host" "$previous")"; then
    echo "deploy/$host already at or past $short"
    return
  fi
  if [ -n "$previous" ] && ! git merge-base --is-ancestor "$(carried "$host" "$previous")" "$sha"; then
    refuse "$host" "not a fast-forward"
    return
  fi
  if ! target=$(promotion "$host" "$previous" 2>/dev/null); then
    refuse "$host" "DEPLOY_SIGNING_KEY cannot sign, it must hold an OpenSSH private key without a passphrase"
    return
  fi
  push_deploy "$host" "$target" "deploy/$host -> $short, signed as $(git rev-parse --short "$target")" "${lease[@]}"
}

while IFS=$'\t' read -r host verdict; do
  if [ "$verdict" != promote ]; then
    echo "deploy/$host $verdict"
    continue
  fi

  tip=$(git rev-parse -q --verify "refs/remotes/origin/deploy/$host" || true)
  if [ "${#signing[@]}" -eq 0 ]; then
    promote_unsigned "$host" "$tip"
  else
    promote_signed "$host" "$tip"
  fi
done <<<"$verdicts"

exit "$refused"
