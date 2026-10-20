state=/var/lib/auto-upgrade/state.json
seen_dir=${XDG_STATE_HOME:-$HOME/.local/state}/upgrade-notify
seen=$seen_dir/seen

if [[ ! -r $state || $(jq -r .outcome "$state") != upgraded ]]; then
  exit 0
fi

upgrade=$(jq -r '"\(.time) \(.to)"' "$state")
if [[ -r $seen && $(<"$seen") == "$upgrade" ]]; then
  exit 0
fi

body=$(jq -r 'if (.changes // "") != "" then .changes else .message end' "$state")
running=$(/run/current-system/sw/bin/nixos-version --configuration-revision 2>/dev/null || true)
if [[ $running != "$(jq -r .to "$state")" ]]; then
  body+=$'\nRestart to use it.'
fi

if ! notify-send --app-name=auto-upgrade "System updated" "$body"; then
  echo "no notification daemon answered; the upgrade is announced at the next session" >&2
  exit 0
fi

mkdir -p "$seen_dir"
printf '%s\n' "$upgrade" >"$seen"
