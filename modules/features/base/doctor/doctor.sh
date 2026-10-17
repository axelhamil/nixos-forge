show_all=false
show_history=false

usage() {
  echo "Usage: doctor [--all | --history]"
  echo "Checks this machine, one line per check: OK, WARN, FAIL or SKIP when it does not apply."
  echo "Run it with sudo for the checks that need root."
  echo "  --all      list every file on / that the next boot erases"
  echo "  --history  list the last 10 runs of auto-upgrade"
}

case "${1:-}" in
"") ;;
--all) show_all=true ;;
--history) show_history=true ;;
-h | --help)
  usage
  exit 0
  ;;
*)
  usage >&2
  exit 2
  ;;
esac

declare -A colors=([OK]="" [WARN]="" [FAIL]="" [SKIP]="" [reset]="")
if [ -t 1 ]; then
  colors=([OK]=$'\e[32m' [WARN]=$'\e[33m' [FAIL]=$'\e[31m' [SKIP]=$'\e[2m' [reset]=$'\e[0m')
fi

failures=0
warnings=0
now=$(date +%s)

is_root() {
  [ "$(id -u)" = 0 ]
}

report() {
  local status=$1 check=$2 detail=$3 hint=${4:-}

  printf '%s[%4s]%s %-14s %s\n' "${colors[$status]}" "$status" "${colors[reset]}" "$check" "$detail"

  if [ -n "$hint" ]; then
    printf '%21s%s\n' "" "$hint"
  fi

  case $status in
  FAIL) failures=$((failures + 1)) ;;
  WARN) warnings=$((warnings + 1)) ;;
  esac
}

detail_line() {
  printf '%21s%s\n' "" "$1"
}

duration() {
  local seconds=$1

  if [ "$seconds" -lt 3600 ]; then
    echo "$((seconds / 60)) min"
  elif [ "$seconds" -lt 172800 ]; then
    echo "$((seconds / 3600)) h"
  else
    echo "$((seconds / 86400)) days"
  fi
}

efi_variable() {
  od -An -tu1 -j4 -N1 "/sys/firmware/efi/efivars/$1-8be4df61-93ca-11d2-aa0d-00e098032b8c" 2>/dev/null | tr -d ' \n' || true
}

check_secure_boot() {
  if [ "$secure_boot" != true ]; then
    report SKIP "Secure Boot" "secure-boot is not imported"
    return
  fi

  if [ ! -d /sys/firmware/efi ]; then
    report FAIL "Secure Boot" "not booted through UEFI" "boot the machine in UEFI mode, not legacy BIOS"
    return
  fi

  local enabled setup
  enabled=$(efi_variable SecureBoot)
  setup=$(efi_variable SetupMode)

  if [ "$setup" = 1 ]; then
    report FAIL "Secure Boot" "firmware in setup mode, keys not enrolled" "reboot: systemd-boot enrolls the keys from /boot/loader/keys/auto"
  elif [ "$enabled" = 1 ]; then
    report OK "Secure Boot" "enforced, keys enrolled"
  else
    report FAIL "Secure Boot" "disabled in the firmware" "turn Secure Boot on in the firmware settings, keeping the enrolled keys"
  fi
}

check_luks() {
  if [ -z "$luks_device" ]; then
    report SKIP "LUKS unlock" "needs disk and secure-boot"
    return
  fi

  if ! is_root; then
    report SKIP "LUKS unlock" "needs root, run sudo doctor"
    return
  fi

  local slots
  if ! slots=$(systemd-cryptenroll "$luks_device" 2>&1); then
    report FAIL "LUKS unlock" "cannot read the keyslots of $luks_device" "$slots"
    return
  fi

  local types
  types=$(awk 'NR > 1 { print $2 }' <<<"$slots")

  if ! grep -qxE 'password|recovery' <<<"$types"; then
    report FAIL "LUKS unlock" "no passphrase keyslot, nothing to fall back on if the TPM2 refuses" "add one now: sudo systemd-cryptenroll --password $luks_device"
  elif ! grep -qx tpm2 <<<"$types"; then
    report WARN "LUKS unlock" "no TPM2 token, the passphrase is asked at every boot" "tpm2-enroll adds one once Secure Boot is enforced: journalctl -b -u tpm2-enroll"
  else
    report OK "LUKS unlock" "TPM2 token, passphrase kept as a fallback"
  fi
}

calendar_interval() {
  local elapses
  mapfile -t elapses < <(TZ=UTC systemd-analyze calendar --iterations=2 "$1" | awk -F ': ' '/Next elapse|Iter\. #2/ { print $2 }')

  if [ "${#elapses[@]}" -lt 2 ]; then
    echo 3600
    return
  fi

  echo $(($(date -d "${elapses[1]}" +%s) - $(date -d "${elapses[0]}" +%s)))
}

unix_property() {
  local value
  value=$(systemctl show --timestamp=unix -p "$1" --value "$2" 2>/dev/null || true)
  value=${value#@}

  if [[ $value =~ ^[0-9]+$ ]]; then
    echo "$value"
  else
    echo 0
  fi
}

check_snapshots() {
  if [ -z "$btrbk_calendar" ]; then
    report SKIP "Snapshots" "no btrbk snapshots on this host"
    return
  fi

  if [ "$(systemctl show -p Result --value btrbk-snapshots.service)" != success ]; then
    report FAIL "Snapshots" "the last btrbk run failed" "journalctl -u btrbk-snapshots"
    return
  fi

  local interval last triggered
  interval=$(calendar_interval "$btrbk_calendar")
  last=$(unix_property ExecMainStartTimestamp btrbk-snapshots.service)
  triggered=$(unix_property LastTriggerUSec btrbk-snapshots.timer)
  if [ "$triggered" -gt "$last" ]; then
    last=$triggered
  fi

  if [ "$last" = 0 ]; then
    report WARN "Snapshots" "btrbk never ran" "systemctl start btrbk-snapshots, then systemctl list-timers btrbk-snapshots"
    return
  fi

  local age=$((now - last))
  if [ "$age" -gt $((2 * interval)) ]; then
    report WARN "Snapshots" "last btrbk run $(duration "$age") ago, scheduled every $(duration "$interval")" "systemctl list-timers btrbk-snapshots"
    return
  fi

  local subvolume newest missing=()
  for subvolume in "${snapshot_subvolumes[@]}"; do
    newest=$(find "$snapshot_dir" -maxdepth 1 -name "$subvolume.*" 2>/dev/null | sort | tail -n 1)
    if [ -z "$newest" ]; then
      missing+=("$subvolume")
    fi
  done

  if [ "${#missing[@]}" -gt 0 ]; then
    report WARN "Snapshots" "no snapshot of ${missing[*]} in $snapshot_dir" "journalctl -u btrbk-snapshots"
  else
    report OK "Snapshots" "last btrbk run $(duration "$age") ago, scheduled every $(duration "$interval")"
  fi
}

check_space() {
  local fstype size avail
  read -r fstype size avail < <(df --output=fstype,size,avail -B1 / | tail -n 1)

  local free=$((avail * 100 / size))
  local left summary
  left=$(numfmt --to=iec "$avail")
  summary="$free % free on / ($fstype, $left left)"

  if [ "$free" -lt 10 ]; then
    report WARN "Disk space" "$summary" "nh clean all --keep 3, and drop old snapshots in /.snapshots"
  else
    report OK "Disk space" "$summary"
  fi
}

failed_units() {
  systemctl "$@" list-units --failed --plain --no-legend --no-pager 2>/dev/null | awk '{ print $1 }' | paste -sd ' ' || true
}

check_units() {
  local system user=""
  system=$(failed_units)

  if ! is_root; then
    user=$(failed_units --user)
  elif [ -n "${SUDO_USER:-}" ]; then
    user=$(failed_units --user --machine="$SUDO_USER@")
  fi

  if [ -z "$system$user" ]; then
    report OK "Failed units" "none"
    return
  fi

  local units=""
  if [ -n "$system" ]; then
    units="system: $system"
  fi
  if [ -n "$user" ]; then
    units="${units:+$units; }user: $user"
  fi

  report FAIL "Failed units" "$units" "systemctl status <unit>, journalctl -b -u <unit> (--user for user units)"
}

as_owner() {
  local owner
  owner=$(stat -c %U "$update_flake")

  if is_root && [ "$owner" != root ]; then
    setpriv --reuid="$owner" --regid="$(id -g "$owner")" --init-groups --reset-env "$(command -v env)" "PATH=$PATH" "$@"
  else
    "$@"
  fi
}

check_updates() {
  if [ -z "$update_flake" ] || [ ! -d "$update_flake/.git" ]; then
    report SKIP "Updates" "the host's flake is not a git checkout on this machine"
    return
  fi

  local fetched=true
  if ! as_owner timeout 30 git -C "$update_flake" fetch --quiet 2>/dev/null; then
    fetched=false
  fi

  local counts
  if ! counts=$(as_owner git -C "$update_flake" rev-list --left-right --count '@{upstream}...HEAD' 2>/dev/null); then
    report WARN "Updates" "cannot compare $update_flake with its upstream branch" "git -C $update_flake branch --set-upstream-to origin/main"
    return
  fi

  local behind ahead
  read -r behind ahead <<<"$counts"

  local state=()
  if [ "$behind" -gt 0 ]; then
    state+=("behind by $behind")
  fi
  if [ "$ahead" -gt 0 ]; then
    state+=("ahead by $ahead")
  fi
  if [ -n "$(as_owner git -C "$update_flake" status --porcelain 2>/dev/null)" ]; then
    state+=("uncommitted changes")
  fi
  if [ "$fetched" = false ]; then
    state+=("fetch failed, counts from the last fetch")
  fi

  local summary
  summary=$(
    IFS=,
    echo "${state[*]}"
  )
  summary=${summary//,/, }

  if [ "$behind" -gt 0 ]; then
    report WARN "Updates" "$update_flake: $summary" "run update"
  elif [ "${#state[@]}" -gt 0 ]; then
    report WARN "Updates" "$update_flake: $summary" "commit and push, so other machines and CI get it"
  else
    report OK "Updates" "$update_flake is up to date with its upstream"
  fi
}

short_date() {
  date -d "$1" '+%Y-%m-%d %H:%M'
}

up_to_date_since() {
  awk -F '\t' '
    $2 == "upgraded" { since = $1 }
    $2 == "up-to-date" && since == "" { since = $1 }
    $2 == "rolled-back" { since = "" }
    END { print since }
  ' "$auto_upgrade_state/history.tsv" 2>/dev/null || true
}

check_upgrades() {
  if [ -z "$auto_upgrade_state" ]; then
    report SKIP "Upgrades" "auto-upgrade is off"
    return
  fi

  local state_file=$auto_upgrade_state/state.json
  if [ ! -r "$state_file" ]; then
    report SKIP "Upgrades" "never ran yet"
    return
  fi

  local fields time outcome message changes epoch
  if ! fields=$(jq -r 'if (.time | type) == "string" and (.outcome | type) == "string" then .time, .outcome, .message // "", .changes // "" else error("missing fields") end' "$state_file" 2>/dev/null) ||
    ! {
      read -r time
      read -r outcome
      read -r message || true
      read -r changes || true
    } <<<"$fields" ||
    ! epoch=$(date -d "$time" +%s 2>/dev/null); then
    report WARN "Upgrades" "cannot read $state_file" "auto-upgrade writes it again at its next run: systemctl start auto-upgrade"
    return
  fi

  local when
  when=$(short_date "$time")

  case $outcome in
  upgraded | up-to-date) ;;
  *)
    report WARN "Upgrades" "$outcome on $when" "$message"
    return
    ;;
  esac

  local age=$((now - epoch))
  if [ "$age" -gt $((14 * 86400)) ]; then
    report WARN "Upgrades" "last run on $when, $(duration "$age") ago" "systemctl list-timers auto-upgrade, journalctl -u auto-upgrade"
  elif [ "$outcome" = upgraded ]; then
    report OK "Upgrades" "upgraded $when: ${changes:-$message}"
  else
    local since
    since=$(up_to_date_since)
    report OK "Upgrades" "up to date since $(short_date "${since:-$time}")"
  fi
}

check_signers() {
  if [ -z "$auto_upgrade_state" ]; then
    report SKIP "Signed deploys" "auto-upgrade is off"
    return
  fi

  if [ "$deploy_signers" = 0 ]; then
    report WARN "Signed deploys" "auto-upgrade applies any deploy commit its sources serve, signed or not" "set the host's signers to the public key of DEPLOY_SIGNING_KEY (update.signers, docs/updates.md)"
    return
  fi

  local keys="$deploy_signers keys"
  if [ "$deploy_signers" = 1 ]; then
    keys="1 key"
  fi
  report OK "Signed deploys" "auto-upgrade applies only deploy commits signed by $keys of update.signers"
}

check_boot() {
  if ! is_root; then
    report SKIP "Boot" "needs root, run sudo doctor"
    return
  fi

  local generation newest
  read -r generation newest < <(bootctl list --json=short --no-pager 2>/dev/null | jq -r '
    map(select((.version // "") | test("Generation [0-9]+")) | .generation = (.version | capture("Generation (?<number>[0-9]+)").number | tonumber))
    | if length == 0 then empty else max_by(.generation) | "\(.generation) \(.path)" end
  ' 2>/dev/null) || true

  if [ -z "${newest:-}" ]; then
    report SKIP "Boot" "no systemd-boot entries on this machine"
  elif [[ $newest =~ \+0-[0-9]+\.(conf|efi)$ ]] && [ "$(readlink -f "/nix/var/nix/profiles/system-$generation-link")" != "$(readlink -f /run/current-system)" ]; then
    local hint="journalctl -b -1 shows why"
    if [ -n "$auto_upgrade_state" ]; then
      hint="$hint, doctor --history lists the upgrades"
    fi
    report WARN "Boot" "the last update did not start, this machine went back to the previous version" "$hint"
  else
    report OK "Boot" "the newest generation started, or has tries left"
  fi
}

show_upgrade_history() {
  if [ -z "$auto_upgrade_state" ]; then
    echo "auto-upgrade is off on this host"
    return
  fi

  local history=$auto_upgrade_state/history.tsv
  if [ ! -s "$history" ]; then
    echo "auto-upgrade never ran yet"
    return
  fi

  tail -n 10 "$history" | awk -F '\t' '
    function cell(value) { return value == "" ? "-" : value }
    BEGIN { printf "%-25s  %-11s  %-12s  %-12s  %s\n", "TIME", "OUTCOME", "FROM", "TO", "SOURCE" }
    { printf "%-25s  %-11s  %-12s  %-12s  %s\n", $1, $2, cell(substr($3, 1, 12)), cell(substr($4, 1, 12)), cell($5) }
  '
}

activation_owned() {
  local clean=()
  if [ -r /etc/.clean ]; then
    mapfile -t clean < <(sed 's|^|/etc/|' /etc/.clean)
  fi

  printf '%s\n' "${clean[@]}" \
    /etc/.clean /etc/.pwd.lock /etc/.updated /var/.updated /etc/NIXOS \
    /etc/passwd /etc/group /etc/shadow /etc/subuid /etc/subgid
}

lost_files() {
  local since=$1 path prune=()
  while read -r path; do
    if [ "$path" != / ]; then
      prune+=(-path "$path" -prune -o)
    fi
  done < <(findmnt --list --noheadings --output TARGET)

  for path in "${ephemeral_paths[@]}"; do
    prune+=(-path "$path" -prune -o)
  done

  find / "${prune[@]}" ! -type d \( -newermt "$since" -o -newerct "$since" \) \
    -printf '%y\t%l\t%p\n' 2>/dev/null |
    awk -F '\t' '
      FNR == NR { owned[$0] = 1; next }
      $1 == "l" && $2 ~ "^(/etc/static|/nix/store)/" { next }
      $1 == "l" && $2 ~ "^(/|(\\.\\./)+)run(/|$)" { next }
      $3 in owned { next }
      { print $3 }
    ' <(activation_owned) - | sort || true
}

check_impermanence() {
  if [ "$wiped_root" != true ]; then
    report SKIP "Impermanence" "/ is not wiped at boot on this host"
    return
  fi

  local since
  since=$(systemctl show --timestamp=us+utc -P ActiveEnterTimestamp sysinit.target)

  local scope=""
  if ! is_root; then
    scope=", without the directories only root reads (run sudo doctor)"
  fi

  local files
  mapfile -t files < <(lost_files "$since")

  if [ "${#files[@]}" = 0 ]; then
    report OK "Impermanence" "nothing written to / since boot is lost at the next boot$scope"
    return
  fi

  report WARN "Impermanence" "${#files[@]} files written to / since boot are lost at the next boot$scope" \
    "keep them in environment.persistence.\"/persist\", or list them in doctor.ephemeralPaths"

  if [ "$show_all" = true ]; then
    printf '%s\n' "${files[@]}" | while read -r path; do detail_line "$path"; done
    return
  fi

  printf '%s\n' "${files[@]}" |
    awk -F / '{
      depth = NF - 1 < 4 ? NF - 1 : 4
      dir = ""
      for (i = 2; i <= depth; i++) dir = dir "/" $i
      count[dir == "" ? "/" : dir]++
    }
    END { for (dir in count) printf "%6d  %s\n", count[dir], dir }' |
    sort -rn | head -n 10 | while read -r line; do detail_line "$line"; done
  detail_line "doctor --all lists every file"
}

if [ "$show_history" = true ]; then
  show_upgrade_history
  exit 0
fi

if ! is_root; then
  echo "Running without root: the checks that need it are skipped. Run sudo doctor for all of them."
  echo
fi

check_secure_boot
check_luks
check_snapshots
check_space
check_units
check_updates
check_upgrades
check_signers
check_boot
check_impermanence

echo
echo "failed: $failures, warnings: $warnings"

if [ "$failures" -gt 0 ]; then
  exit 1
fi
