esp_label=${ESP_DEVICE##*/}
luks_label=${LUKS_DEVICE##*/}
esp_type=c12a7328-f81f-11d2-ba4b-00a0c93ec93b
mib=$((1024 * 1024))
generations=3
margin=$((32 * mib))
store_margin=$((1024 * mib))

usage() {
  cat <<EOF
Usage: install-beside-windows DISK SYSTEM

Creates the encrypted NixOS partition in the free space of DISK, which holds
Windows, and mounts it on /mnt with the Windows ESP on /mnt/boot. The Windows
partitions and their content are kept; only the GPT name of the ESP changes,
to $esp_label, so the installed system finds it.
SYSTEM is the system to install: it sets the room needed on the ESP and the disk.
EOF
}

fail() {
  printf '\ninstall-beside-windows: %s\n' "$*" >&2
  exit 1
}

say() {
  printf '%s\n' "$*" >&2
}

[ $# -eq 2 ] || {
  usage >&2
  exit 2
}
disk=$(realpath "$1")
system=$2
[ -b "$disk" ] || fail "$1 is not a block device."
[ -e "$system/kernel" ] && [ -e "$system/initrd" ] || fail "$system is not a NixOS system."

partitions() {
  lsblk -nrpo NAME,PARTTYPE,PARTLABEL "$disk" | awk 'NF >= 2'
}

partition_number() {
  cat "/sys/class/block/$(basename "$1")/partition"
}

mapfile -t esps < <(partitions | awk -v type="$esp_type" 'tolower($2) == type { print $1 }')
[ ${#esps[@]} -gt 0 ] || fail "$disk has no EFI system partition, so no Windows to install beside. Install on a whole disk instead, or check the disk name."
[ ${#esps[@]} -eq 1 ] || fail "$disk has ${#esps[@]} EFI system partitions (${esps[*]}): keep one, then run again."
esp=${esps[0]}

esp_dir=$(mktemp -d)
trap 'umount "$esp_dir" 2>/dev/null || true; rmdir "$esp_dir"' EXIT
mount -o ro "$esp" "$esp_dir" || fail "cannot mount the ESP $esp."
[ -e "$esp_dir/EFI/Microsoft/Boot/bootmgfw.efi" ] ||
  fail "the ESP $esp holds no EFI/Microsoft/Boot/bootmgfw.efi: Windows is not installed on $disk."

available=$(df -B1 --output=avail "$esp_dir" | tail -n 1)
reused=$({ du -sbc "$esp_dir/EFI/nixos" "$esp_dir/EFI/Linux" "$esp_dir/loader" 2>/dev/null || true; } | tail -n 1 | cut -f 1)
umount "$esp_dir"

generation_size=$(($(stat -L -c %s "$system/kernel") + $(stat -L -c %s "$system/initrd")))
needed=$((generations * generation_size + margin))
free=$((available + ${reused:-0}))
if [ "$free" -lt "$needed" ]; then
  fail "the ESP $esp has $((free / mib)) MiB free, and this system needs $((needed / mib)) MiB to keep $generations generations ($((generation_size / mib)) MiB each).
Grow the ESP to 1 GiB first: shrink the Windows partition in Windows Disk Management, then move it and grow the ESP with GParted from a live USB.
Otherwise install NixOS on its own disk, which gets its own ESP."
fi

luks=$(partitions | awk -v label="$luks_label" '$3 == label { print $1 }')
if [ -n "$luks" ]; then
  say "Reusing $luks, the NixOS partition of a previous install: its content is erased."
else
  closure=$(nix-store --query --requisites "$system" | xargs nix-store --query --size | awk '{ total += $1 } END { print total }')
  needed_space=$((closure + store_margin))
  start=$(sgdisk --first-aligned-in-largest "$disk" | tail -n 1)
  end=$(sgdisk --end-of-largest "$disk" | tail -n 1)
  sector=$(blockdev --getss "$disk")
  largest=$(((end - start + 1) * sector))
  if [ "$start" -ge "$end" ] || [ "$largest" -lt "$needed_space" ]; then
    fail "$disk has $((largest > 0 ? largest / mib : 0)) MiB of free space in one block, and NixOS needs at least $((needed_space / mib)) MiB.
Shrink the Windows partition in Windows Disk Management (Shrink Volume), then run again."
  fi
  number=$(sgdisk --print "$disk" | awk '$1 ~ /^[0-9]+$/ { last = $1 } END { print last + 1 }')
  sgdisk --new="$number:$start:$end" --change-name="$number:$luks_label" --typecode="$number:8309" "$disk" >/dev/null
  say "Created partition $number of $disk, $((largest / mib)) MiB, for NixOS."
fi

if [ "$(lsblk -nro PARTLABEL "$esp")" != "$esp_label" ]; then
  sgdisk --change-name="$(partition_number "$esp"):$esp_label" "$disk" >/dev/null
  say "Named the ESP $esp $esp_label in the partition table."
fi

partprobe "$disk"
udevadm settle
[ -e "$LUKS_DEVICE" ] && [ -e "$ESP_DEVICE" ] ||
  fail "the kernel does not show the new partition labels yet: reboot the installer and run again."
wipefs -a "$LUKS_DEVICE" >/dev/null

"$FORMAT_MOUNT"
