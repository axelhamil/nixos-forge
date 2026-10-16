if [ "$(id -u)" -ne 0 ]; then
  exec sudo "$0" "$@"
fi

disk=""
target_device=""
passphrase_file=""
passphrase=""
assume_yes=false
reboot_after=true
declare -A password_files=()
declare -A password_hashes=()

usage() {
  cat <<EOF
Usage: install-host [--disk DEVICE] [--passphrase-file FILE] [--password-file USER=FILE]... [--yes] [--no-reboot]

Installs $HOST_NAME on a whole disk and erases everything on it.
Every value not given on the command line is asked for, unless --yes is set.

  --disk DEVICE              disk to erase, such as /dev/nvme0n1
  --passphrase-file FILE     file holding the disk encryption passphrase
  --password-file USER=FILE  file holding the login password of USER
  --yes                      ask nothing: erase without confirmation, fail on a missing value, reboot at the end
  --no-reboot                stay on the installer once the install is done
EOF
}

say() {
  printf '\n%s\n' "$*" >&2
}

fail() {
  printf '\ninstall-host: %s\n' "$*" >&2
  exit 1
}

while [ $# -gt 0 ]; do
  case $1 in
    --disk)
      disk=${2:?--disk needs a device}
      shift 2
      ;;
    --passphrase-file)
      passphrase_file=${2:?--passphrase-file needs a file}
      shift 2
      ;;
    --password-file)
      entry=${2:?--password-file needs USER=FILE}
      password_files[${entry%%=*}]=${entry#*=}
      shift 2
      ;;
    --yes)
      assume_yes=true
      shift
      ;;
    --no-reboot)
      reboot_after=false
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      usage >&2
      exit 2
      ;;
  esac
done

read_file() {
  local content
  [ -r "$1" ] || fail "cannot read $1."
  content=$(<"$1")
  [ -n "$content" ] || fail "$1 is empty: it must hold $2."
  printf '%s' "$content"
}

live_media() {
  local source backing
  source=$(findmnt -no SOURCE /iso 2>/dev/null) || return 0
  if backing=$(losetup -nO BACK-FILE "$source" 2>/dev/null) && [ -n "$backing" ]; then
    source=$(findmnt -no SOURCE -T "$backing" 2>/dev/null) || return 0
  fi
  [ -b "$source" ] || return 0
  lsblk -nprs -o NAME "$source"
}

candidate_disks() {
  local live
  live=$(live_media)
  lsblk -J -d -p -b -o NAME,TYPE,SIZE,RO |
    jq -r '.blockdevices[] | select(.type == "disk" and .ro == false and .size > 0 and (.name | test("^/dev/(fd|zram)[0-9]") | not)) | .name' |
    grep -vxF -e "$live" -e "" || true
}

describe() {
  local size model
  size=$(lsblk -dno SIZE "$1" | tr -d ' ')
  model=$(lsblk -dno MODEL "$1" | sed 's/ *$//')
  printf '%s, %s' "$size" "${model:-unknown model}"
}

choose_disk() {
  local disks choice index
  mapfile -t disks < <(candidate_disks)
  [ ${#disks[@]} -gt 0 ] || fail "no disk to install on: plug one in and run install-host again."

  say "Disks:"
  for index in "${!disks[@]}"; do
    printf '  %d) %s  (%s)\n' $((index + 1)) "${disks[index]}" "$(describe "${disks[index]}")" >&2
  done
  echo >&2

  while true; do
    read -rp "Disk to install on, 1 to ${#disks[@]}: " choice
    if [[ $choice =~ ^[0-9]+$ ]] && [ "$choice" -ge 1 ] && [ "$choice" -le ${#disks[@]} ]; then
      disk=${disks[choice - 1]}
      return
    fi
    echo "Type a number from the list." >&2
  done
}

check_disk() {
  local others
  [ -b "$disk" ] || fail "$disk is not a block device."
  target_device=$(realpath "$disk")
  candidate_disks | grep -qxF "$target_device" ||
    fail "$disk is not a whole writable disk, or it holds this installer."

  others=$(lsblk -nrpo NAME,PKNAME,PARTLABEL | awk -v target="$target_device" '$3 ~ /^disk-main-/ && $2 != target { print $1 }')
  [ -z "$others" ] ||
    fail "$(echo "$others" | tr '\n' ' ')already carries this layout's partition labels, so the install could format it instead of $disk. Unplug that disk or erase its partitions (wipefs -a), then run install-host again."
}

read_secret() {
  local first second
  while true; do
    read -rsp "$1: " first
    echo >&2
    read -rsp "Same again: " second
    echo >&2
    if [ -z "$first" ]; then
      echo "It cannot be empty." >&2
    elif [ "$first" != "$second" ]; then
      echo "They differ, try again." >&2
    else
      printf '%s' "$first"
      return
    fi
  done
}

hash_password() {
  printf '%s' "$1" | mkpasswd --method=yescrypt --stdin
}

confirm_erase() {
  local typed
  say "Ready to install"
  printf '  Host:   %s\n  Disk:   %s (%s), erased entirely\n  Users:  %s\n\n' \
    "$HOST_NAME" "$disk" "$(describe "$disk")" "${USERS[*]}" >&2
  read -rp "Type $disk to erase it and install: " typed
  [ "$typed" = "$disk" ] || fail "the name did not match, nothing was erased."
}

release_target() {
  umount -R /mnt 2>/dev/null || true
  cryptsetup close "$LUKS_NAME" 2>/dev/null || true
}

cleanup() {
  rm -f "$PASSPHRASE_FILE" "$TARGET_LINK"
  release_target
}

for user in "${!password_files[@]}"; do
  [[ " ${USERS[*]} " == *" $user "* ]] || fail "$user is not a user of $HOST_NAME (${USERS[*]})."
done

say "Install $HOST_NAME"
echo "This installer erases one whole disk and installs $HOST_NAME on it, encrypted." >&2

if [ -z "$disk" ]; then
  [ "$assume_yes" != true ] || fail "--yes needs --disk."
  choose_disk
fi
check_disk

if [ -n "$passphrase_file" ]; then
  passphrase=$(read_file "$passphrase_file" "the disk passphrase")
elif [ "$assume_yes" = true ]; then
  fail "--yes needs --passphrase-file."
else
  passphrase=$(read_secret "Disk passphrase, asked at boot to unlock it")
fi

for user in "${USERS[@]}"; do
  if [ -n "${password_files[$user]:-}" ]; then
    password=$(read_file "${password_files[$user]}" "the password of $user")
  elif [ "$assume_yes" = true ]; then
    fail "--yes needs --password-file $user=FILE."
  else
    password=$(read_secret "Login password of $user")
  fi
  password_hashes[$user]=$(hash_password "$password")
done

if [ "$assume_yes" != true ]; then
  confirm_erase
fi

trap cleanup EXIT
release_target
mkdir -p "$(dirname "$TARGET_LINK")"
ln -sfn "$target_device" "$TARGET_LINK"
(
  umask 077
  printf '%s' "$passphrase" >"$PASSPHRASE_FILE"
)

say "Erasing $disk and creating the encrypted layout"
"$DISKO" --yes-wipe-all-disks

install -d -m 700 /mnt/persist/passwords
for user in "${USERS[@]}"; do
  install -m 600 /dev/null "/mnt/persist/passwords/$user"
  printf '%s\n' "${password_hashes[$user]}" >"/mnt/persist/passwords/$user"
done

say "Recording the hardware report in /persist/facter.json"
nixos-facter -o /mnt/persist/facter.json || echo "No hardware report: nixos-facter failed, the install goes on." >&2

say "Installing $HOST_NAME"
nixos-install --system "$SYSTEM" --no-root-passwd --no-channel-copy
release_target

say "$HOST_NAME is installed on $disk."
if [ "$reboot_after" != true ]; then
  exit 0
fi

if [ "$assume_yes" != true ]; then
  read -rp "Remove the installer medium, then reboot now? [Y/n] " answer
  case $answer in
    [nN]*)
      echo "Reboot when ready with: systemctl reboot" >&2
      exit 0
      ;;
  esac
fi
systemctl reboot
