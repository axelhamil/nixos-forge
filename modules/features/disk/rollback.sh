mkdir -p /btrfs
mount -t btrfs -o subvol=/ "$device" /btrfs

if btrfs subvolume show /btrfs/@root >/dev/null 2>&1; then
  btrfs subvolume delete --recursive /btrfs/@root
fi

btrfs subvolume snapshot /btrfs/@root-blank /btrfs/@root

umount /btrfs
