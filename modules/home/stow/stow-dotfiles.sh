usage() {
  echo "usage: stow-dotfiles prune|check|apply <dotfiles> <target> <state> [--dotfiles] [package...]" >&2
  exit 2
}

[ $# -ge 4 ] || usage
mode=$1 directory=$2 target=$3 state=$4
shift 4

flags=(--no-folding --target "$target")
if [ "${1:-}" = --dotfiles ]; then
  flags+=(--dotfiles)
  shift
fi

if [ ! -d "$directory" ]; then
  [ "$mode" = apply ] && echo "stow: no dotfiles in $directory yet; clone them there, then run: systemctl restart home-manager-$USER"
  exit 0
fi

mirrors_home() {
  find "$directory" -mindepth 1 -maxdepth 1 -name '.*' ! -name .git ! -name '.stow*' -print -quit | grep -q .
}

if [ $# -gt 0 ]; then
  requested=("$@")
elif mirrors_home; then
  requested=(.)
else
  mapfile -t requested < <(find "$directory" -mindepth 1 -maxdepth 1 -type d ! -name '.*' -printf '%f\n' | sort)
fi

stow_package() {
  local package=$1
  shift
  if [ "$package" = . ]; then
    local mirror=${state%/*}/mirror
    mkdir -p "$mirror"
    ln -sfn "$directory" "$mirror/home"
    stow "${flags[@]}" --dir "$mirror" "$@" home
  else
    stow "${flags[@]}" --dir "$directory" "$@" "$package"
  fi
}

declare -A wanted
packages=()
for package in "${requested[@]}"; do
  if [ "$package" = . ] || [ -d "$directory/$package" ]; then
    packages+=("$package")
    wanted[$package]=1
  else
    echo "stow: no package $package in $directory, skipped" >&2
  fi
done

prune() {
  local previous=() package
  [ -f "$state" ] && mapfile -t previous <"$state"

  for package in "${previous[@]}"; do
    [ -n "$package" ] && [ -z "${wanted[$package]:-}" ] || continue
    if [ "$package" = . ] || [ -d "$directory/$package" ]; then
      stow_package "$package" --delete
    fi
  done
}

check() {
  local package report
  for package in "${packages[@]}"; do
    report=$(stow_package "$package" --simulate 2>&1) && continue
    if grep -q 'existing target' <<<"$report"; then
      echo "stow: $package has files that home-manager also writes:" >&2
      grep -o 'existing target .*' <<<"$report" >&2
      echo "stow: turn off the module that writes them in your home config (for instance programs.git.enable = false), or remove them from your dotfiles" >&2
    else
      echo "$report" >&2
    fi
    return 1
  done
}

apply() {
  local package
  for package in "${packages[@]}"; do
    stow_package "$package" --restow ||
      echo "stow: $package not linked: move the files listed above out of the way, then run: systemctl restart home-manager-$USER" >&2
  done

  mkdir -p "${state%/*}"
  if [ ${#packages[@]} -gt 0 ]; then
    printf '%s\n' "${packages[@]}" >"$state"
  else
    : >"$state"
  fi
}

case $mode in
prune) prune ;;
check) check ;;
apply) apply ;;
*) usage ;;
esac
