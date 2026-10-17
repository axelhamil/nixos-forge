mkdir -p /btrfs
mount -t btrfs -o subvol=/ "$device" /btrfs

if btrfs subvolume show /btrfs/@home >/dev/null 2>&1; then
  if [ -d /btrfs/@snapshots ]; then
    stamp="/btrfs/@snapshots/home-wiped.$(date -u +%Y%m%dT%H%M%S)"
    wiped=$stamp
    copies=1
    while [ -e "$wiped" ]; do
      wiped="${stamp}_$copies"
      copies=$((copies + 1))
    done
    mv /btrfs/@home "$wiped"
    btrfs property set -ts "$wiped" ro true
  else
    btrfs subvolume delete --recursive /btrfs/@home
  fi
fi

btrfs subvolume create /btrfs/@home

expired=$(date -u -d "-7 days" +%Y%m%dT%H%M%S)
newest_kept=2
mapfile -t wiped_homes < <(printf '%s\n' /btrfs/@snapshots/home-wiped.* | sort)
prunable=$((${#wiped_homes[@]} - newest_kept))

for snapshot in "${wiped_homes[@]:0:$((prunable > 0 ? prunable : 0))}"; do
  btrfs subvolume show "$snapshot" >/dev/null 2>&1 || continue
  [[ ${snapshot##*/home-wiped.} < "$expired" ]] || continue
  {
    btrfs property set -ts "$snapshot" ro false &&
      btrfs subvolume delete --recursive "$snapshot"
  } || echo "Could not delete $snapshot, left in place" >&2
done

umount /btrfs
