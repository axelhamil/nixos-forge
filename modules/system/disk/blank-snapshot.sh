mountpoint=$(mktemp -d)
mount -o subvol=/ "$device" "$mountpoint"

if ! btrfs subvolume show "$mountpoint/@root-blank" >/dev/null 2>&1; then
  btrfs subvolume snapshot -r "$mountpoint/@root" "$mountpoint/@root-blank"
fi

umount "$mountpoint"
rmdir "$mountpoint"
