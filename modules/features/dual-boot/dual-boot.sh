esp_type=c12a7328-f81f-11d2-ba4b-00a0c93ec93b
boot_manager='EFI/Microsoft/Boot/bootmgfw.efi'
copy=$ESP_MOUNT/$boot_manager
copy_mark=$ESP_MOUNT/EFI/Microsoft/Boot/copied-by-nixos
linux_loader='\EFI\systemd\systemd-bootx64.efi'
windows_loader='\EFI\Microsoft\Boot\bootmgfw.efi'

warn() {
  echo "dual-boot: $*" >&2
}

our_esp=$(findmnt -nvo SOURCE --mountpoint "$ESP_MOUNT" 2>/dev/null) || {
  warn "no ESP is mounted on $ESP_MOUNT, nothing to do."
  exit 0
}
our_esp=$(realpath "$our_esp")

other_esps() {
  lsblk -nrpo NAME,PARTTYPE | awk -v type="$esp_type" -v ours="$our_esp" 'tolower($2) == type && $1 != ours { print $1 }'
}

copy_boot_manager() {
  local esp=$1 dir status=1
  dir=$(mktemp -d)
  if ! mount -o ro,nodev,nosuid,noexec "$esp" "$dir" 2>/dev/null; then
    rmdir "$dir"
    return 1
  fi
  if [ -f "$dir/$boot_manager" ]; then
    status=0
    if cmp -s "$dir/$boot_manager" "$copy"; then
      :
    elif mkdir -p "$(dirname "$copy")" && cp "$dir/$boot_manager" "$copy.new" && mv "$copy.new" "$copy" && touch "$copy_mark"; then
      echo "dual-boot: copied the Windows boot manager of $esp to $copy, so the menu shows Windows." >&2
    else
      warn "could not copy the Windows boot manager of $esp to $ESP_MOUNT."
    fi
  fi
  umount "$dir"
  rmdir "$dir"
  return $status
}

windows_esp=""
if [ -f "$copy" ] && [ ! -e "$copy_mark" ]; then
  windows_esp=$our_esp
else
  while read -r esp; do
    if copy_boot_manager "$esp"; then
      windows_esp=$esp
      break
    fi
  done < <(other_esps)

  if [ -z "$windows_esp" ] && [ -e "$copy_mark" ]; then
    rm -f "$copy" "$copy_mark"
    rmdir -p "$(dirname "$copy")" 2>/dev/null || true
    warn "no other disk holds Windows any more: removed the copy of its boot manager."
  fi
fi

if [ -z "$windows_esp" ]; then
  exit 0
fi

if [ ! -d /sys/firmware/efi/efivars ]; then
  exit 0
fi

entry_number() {
  local label=$1 line
  while IFS=$'\t' read -r line _; do
    if [[ $line =~ ^Boot([0-9A-Fa-f]{4})\*?\ (.*)$ ]] && [ "${BASH_REMATCH[2]}" = "$label" ]; then
      printf '%s\n' "${BASH_REMATCH[1]}"
      return 0
    fi
  done < <(efibootmgr 2>/dev/null)
  return 1
}

ensure_entry() {
  local label=$1 loader=$2 esp=$3 name disk part
  entry_number "$label" && return 0
  name=$(basename "$esp")
  disk=/dev/$(lsblk -ndo PKNAME "$esp") &&
    part=$(cat "/sys/class/block/$name/partition") &&
    efibootmgr --create-only --disk "$disk" --part "$part" --label "$label" --loader "$loader" >/dev/null ||
    return 1
  echo "dual-boot: created the firmware entry $label." >&2
  entry_number "$label"
}

order_entries() {
  local linux windows current wanted entry
  linux=$(ensure_entry "Linux Boot Manager" "$linux_loader" "$our_esp") || return 1
  windows=$(ensure_entry "Windows Boot Manager" "$windows_loader" "$windows_esp") || return 1
  current=$(efibootmgr | sed -n 's/^BootOrder: //p')
  wanted="$linux,$windows"
  IFS=, read -ra entries <<<"$current"
  for entry in "${entries[@]}"; do
    if [ "$entry" != "$linux" ] && [ "$entry" != "$windows" ]; then
      wanted+=",$entry"
    fi
  done
  if [ "$wanted" != "$current" ]; then
    efibootmgr --bootorder "$wanted" >/dev/null || return 1
    echo "dual-boot: firmware boot order set to $wanted, NixOS first and Windows next." >&2
  fi
}

order_entries || warn "could not set the firmware boot entries: pick Windows in the menu once its firmware entry exists."
exit 0
