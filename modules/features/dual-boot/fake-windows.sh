disk=$1
esp_size=$2
esp_used=$3

sgdisk --zap-all "$disk" >/dev/null
sgdisk \
  --new="1:0:+$esp_size" --typecode=1:ef00 --change-name="1:EFI system partition" \
  --new=2:0:+16M --typecode=2:0c01 --change-name="2:Microsoft reserved partition" \
  --new=3:0:+512M --typecode=3:0700 --change-name="3:Basic data partition" \
  --new=4:-64M:0 --typecode=4:2700 --change-name=4:Recovery \
  "$disk" >/dev/null
partprobe "$disk"
udevadm settle

part() {
  lsblk -nrpo NAME "$disk" | sed -n "$(($1 + 1))p"
}

mkfs.vfat -F 32 -n SYSTEM "$(part 1)" >/dev/null
mkntfs -Q -L Windows "$(part 3)" >/dev/null

root=$(mktemp -d)
mount "$(part 1)" "$root"
boot=$root/EFI/Microsoft/Boot
mkdir -p "$boot"
if [ -d /var/lib/sbctl/keys ]; then
  sbctl sign -o "$boot/bootmgfw.efi" "$WINDOWS_EFI" >/dev/null
else
  cp "$WINDOWS_EFI" "$boot/bootmgfw.efi"
fi
cp "$STARTUP_SCRIPT" "$boot/startup.nsh"
if [ "$esp_used" -gt 0 ]; then
  dd if=/dev/urandom of="$boot/fonts.bin" bs=1M count="$esp_used" status=none
fi
umount "$root"

mount "$(part 3)" "$root"
mkdir -p "$root/Users/Public"
echo "hello from windows" >"$root/Users/Public/hello.txt"
umount "$root"
rmdir "$root"
sync
