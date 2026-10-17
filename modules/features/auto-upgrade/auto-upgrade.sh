state_dir=/var/lib/auto-upgrade
repo=$state_dir/repo
branch=deploy/$HOST
now=$(date --iso-8601=seconds)
from=""
to=""
source=""
changes=""

finish() {
  local outcome=$1 message=$2
  jq -n \
    --arg time "$now" \
    --arg outcome "$outcome" \
    --arg from "$from" \
    --arg to "$to" \
    --arg source "$source" \
    --arg message "$message" \
    --arg changes "$changes" \
    '{ $time, $outcome, $from, $to, $source, $message, $changes }' >"$state_dir/state.json.new"
  mv "$state_dir/state.json.new" "$state_dir/state.json"
  printf '%s\t%s\t%s\t%s\t%s\n' "$now" "$outcome" "$from" "$to" "$source" >>"$state_dir/history.tsv"
  printf '%s: %s\n' "$outcome" "$message"
}

admin() {
  [[ $1 == root ]] || id -nG -- "$1" 2>/dev/null | tr ' ' '\n' | grep -qx wheel
}

over_https() {
  local url=$1
  if [[ $url =~ ^ssh://([^@/]+@)?([^/:]+)(:[0-9]+)?/(.+)$ ]]; then
    printf 'https://%s/%s\n' "${BASH_REMATCH[2]}" "${BASH_REMATCH[4]}"
  elif [[ $url != *://* && $url =~ ^([^@/:]+@)?([^/:]+):(.+)$ ]]; then
    printf 'https://%s/%s\n' "${BASH_REMATCH[2]}" "${BASH_REMATCH[3]#/}"
  else
    printf '%s\n' "$url"
  fi
}

revision_of() {
  local system=$1
  if [[ -x $system/sw/bin/nixos-version ]]; then
    "$system/sw/bin/nixos-version" --configuration-revision 2>/dev/null || true
  fi
}

next_did_not_start() {
  if [[ -n ${NEXT_DID_NOT_START:-} ]]; then
    [[ $NEXT_DID_NOT_START == true ]]
    return
  fi
  local profile generation
  [[ $(readlink -f /run/current-system) != "$(readlink -f /nix/var/nix/profiles/system)" ]] || return 1
  profile=$(readlink /nix/var/nix/profiles/system)
  [[ $profile =~ ^system-([0-9]+)-link$ ]] || return 1
  generation=${BASH_REMATCH[1]}
  bootctl list --json=short --no-pager 2>/dev/null |
    jq -e --arg generation "$generation" \
      'any(.[]; ((.version // "") | test("\\bGeneration \($generation)\\b")) and (.path | test("\\+0-[0-9]+\\.(conf|efi)$")))' >/dev/null
}

refuse_checkout() {
  finish refused "the checkout $CHECKOUT must be owned by root or an admin (wheel), writable by no one outside root and wheel, with a plain .git folder, for auto-upgrade to trust its origin remote"
  exit 0
}

trusted() {
  local owner group mode writers=8#022
  read -r owner group mode < <(stat -L -c '%U %G %a' -- "$1") || return 1
  if [[ $group == wheel || $group == root ]]; then
    writers=8#002
  fi
  admin "$owner" && (((8#$mode & writers) == 0))
}

sources=()
if [[ -n $CHECKOUT && -d $CHECKOUT ]]; then
  exec {checkout_fd}<"$CHECKOUT"
  trusted "/proc/self/fd/$checkout_fd" || refuse_checkout
  git_dir=/proc/self/fd/$checkout_fd/.git
  if [[ -e $git_dir || -L $git_dir ]]; then
    [[ -d $git_dir && ! -L $git_dir ]] || refuse_checkout
    exec {git_fd}<"$git_dir"
    trusted "/proc/self/fd/$git_fd" || refuse_checkout
    config=/proc/self/fd/$git_fd/config
    [[ -f $config && ! -L $config ]] || refuse_checkout
    exec {config_fd}<"$config"
    trusted "/proc/self/fd/$config_fd" || refuse_checkout
    if origin=$(git config --file "/dev/fd/$config_fd" --get remote.origin.url); then
      sources+=("$(over_https "$origin")")
    fi
  fi
fi
while IFS= read -r line; do
  [[ -n $line ]] && sources+=("$line")
done <<<"$SOURCES"

[[ -d $repo ]] || git init --quiet --bare "$repo"

answered=false
for index in "${!sources[@]}"; do
  candidate=${sources[index]}
  status=0
  timeout 60 git ls-remote --quiet --exit-code --end-of-options "$candidate" "refs/heads/$branch" >/dev/null || status=$?
  if ((status == 2)); then
    answered=true
    continue
  fi
  ((status == 0)) || continue
  if timeout 60 git -C "$repo" fetch --quiet --no-tags --end-of-options "$candidate" "+refs/heads/$branch:refs/candidates/$index"; then
    source=$candidate
    to=$(git -C "$repo" rev-parse "refs/candidates/$index")
    break
  fi
done

if [[ -z $source && $answered == true ]]; then
  finish waiting "CI has not validated a version for this host yet"
  exit 0
fi

if [[ -z $source ]]; then
  finish unreachable "no source reachable: this machine stays on its version"
  exit 0
fi

running=${CURRENT_REV:-$(revision_of /run/current-system)}
next=${NEXT_REV:-$(revision_of /nix/var/nix/profiles/system)}
next=${next:-$running}
from=$running
failed=""
if [[ $next != "$running" ]] && next_did_not_start; then
  failed=$next
  next=$running
fi

for revision in "$running" "$next"; do
  if [[ -z $revision || $revision == *-dirty ]]; then
    finish refused "this machine runs uncommitted changes: commit and push them, CI will validate them"
    exit 0
  fi
done

if [[ $next == "$to" && $running == "$to" ]]; then
  finish up-to-date "$branch is the running commit"
  exit 0
fi

if [[ -n $failed && $failed == "$to" ]]; then
  finish rolled-back "$(git -C "$repo" rev-parse --short "$to") did not start and the machine went back to the previous generation: fix it and push, CI will validate the fix"
  exit 0
fi

if [[ $next == "$to" ]]; then
  finish up-to-date "$branch is installed and applies at next boot"
  exit 0
fi

for revision in "$running" "$next"; do
  if ! git -C "$repo" merge-base --is-ancestor "$revision" "$to" 2>/dev/null; then
    finish refused "$branch does not contain the running commit: push it, or the pointer was moved back"
    exit 0
  fi
done

build_options=()
if [[ $ALLOW_LOCAL_BUILD != true ]]; then
  build_options+=(--option max-jobs 0 --option always-allow-substitutes true)
fi

old_system=$(readlink -f /run/current-system)

if ! "$REBUILD" "$OPERATION" --flake "git+file://$repo?rev=$to#$HOST" "${build_options[@]}" 2>&1 | tee "$state_dir/rebuild.log"; then
  if grep -q -e "local builds are disabled" -e "Unable to start any build" "$state_dir/rebuild.log"; then
    finish waiting "the cache does not have this version yet"
    exit 0
  fi
  finish failed "nixos-rebuild $OPERATION failed: journalctl -u auto-upgrade shows why"
  exit 1
fi

changes=$(nvd diff "$old_system" /nix/var/nix/profiles/system 2>/dev/null | grep '^Closure size:' || true)

if [[ $OPERATION == boot ]]; then
  finish upgraded "applies at next boot"
else
  finish upgraded "running now"
fi
