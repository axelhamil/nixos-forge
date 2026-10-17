usage() {
  cat <<'EOF_USAGE'
Usage: nix run .#try-install -- <host> [--disk-size 32G] [--reset]

Installs <host> from its installer ISO in a local QEMU window, as on a new machine: UEFI firmware
with Secure Boot in setup mode, a software TPM2 and a blank disk image. Run it from your fork:
it builds .#installer-<host> first. The disk image, the firmware variables and the TPM state
live in $XDG_STATE_HOME/try-install/<host>/, so running it again boots the installed system.

  --disk-size SIZE   size of the disk image when it is created (default 32G)
  --reset            erase the disk, the firmware variables and the TPM, then start again
  -h, --help         show this help

It never touches a real disk: to install on a machine, boot the ISO on it (docs/install.md).
EOF_USAGE
}

die() {
  printf 'try-install: %s\n' "$*" >&2
  exit 1
}

for arg in "$@"; do
  [[ $arg != /dev/* ]] || die "$arg looks like a device: try-install never touches a real disk, it installs on a disk image. To install on a machine, boot the ISO on it (docs/install.md)."
done

host="" disk_size=32G reset=false
while (($# > 0)); do
  case $1 in
  -h | --help)
    usage
    exit 0
    ;;
  --reset) reset=true ;;
  --disk-size)
    (($# > 1)) || die "--disk-size takes a size, such as 32G"
    disk_size=$2
    shift
    ;;
  -*) die "unknown option $1 (see --help)" ;;
  *)
    [[ -z $host ]] || die "one host only, got $host and $1"
    host=$1
    ;;
  esac
  shift
done

[[ $disk_size =~ ^[0-9]+[KMGT]?$ ]] || die "--disk-size takes a size, such as 32G, got $disk_size"
[[ -n $host ]] || die "name a host (see --help)"

read -r -a installer_hosts <<<"$INSTALLER_HOSTS"
known=false
for name in "${installer_hosts[@]}"; do
  [[ $name != "$host" ]] || known=true
done
$known || die "no installer for $host, hosts with one: ${INSTALLER_HOSTS// /, } (a host gets one when it lists \"disk\" in features)"

[[ -w /dev/kvm ]] || die "/dev/kvm is missing or not writable: enable virtualisation in the firmware and join the kvm group, since an install without KVM takes hours"

state=${XDG_STATE_HOME:-$HOME/.local/state}/try-install/$host

iso_out=$(nix build --no-link --print-out-paths ".#installer-$host")
isos=("$iso_out"/iso/*.iso)
[[ -f ${isos[0]} ]] || die "no ISO in $iso_out/iso"

if $reset; then
  rm -rf -- "$state"
fi

umask 077
mkdir -p "$state/swtpm"
cp --no-preserve=mode --update=none "$FIRMWARE_VARS" "$state/vars.fd"
[[ -e $state/disk.qcow2 ]] || qemu-img create -q -f qcow2 "$state/disk.qcow2" "$disk_size"

runtime=$(mktemp -d "${XDG_RUNTIME_DIR:-/tmp}/try-install.XXXXXX")
stop_tpm() {
  [[ ! -f $runtime/swtpm.pid ]] || kill "$(<"$runtime/swtpm.pid")" 2>/dev/null || true
  rm -rf -- "$runtime"
}
trap stop_tpm EXIT

swtpm socket \
  --tpmstate dir="$state/swtpm" \
  --ctrl type=unixio,path="$runtime/swtpm.sock" \
  --pid file="$runtime/swtpm.pid" \
  --tpm2 \
  --terminate \
  --daemon

for _ in {1..100}; do
  [[ ! -S $runtime/swtpm.sock ]] || break
  sleep 0.1
done
[[ -S $runtime/swtpm.sock ]] || die "swtpm did not open its socket within 10 seconds"

echo "try-install: $host, state in $state"
qemu-system-x86_64 \
  -name "try-install $host" \
  -machine q35,smm=on,accel=kvm \
  -cpu max \
  -smp 4 \
  -m 4096 \
  -global driver=cfi.pflash01,property=secure,value=on \
  -drive if=pflash,format=raw,unit=0,readonly=on,file="$FIRMWARE" \
  -drive if=pflash,format=raw,unit=1,file="$state/vars.fd" \
  -chardev socket,id=chrtpm,path="$runtime/swtpm.sock" \
  -tpmdev emulator,id=tpm0,chardev=chrtpm \
  -device tpm-tis,tpmdev=tpm0 \
  -drive if=none,id=disk,format=qcow2,file="$state/disk.qcow2" \
  -device virtio-blk-pci,drive=disk,bootindex=1 \
  -drive if=none,id=iso,media=cdrom,readonly=on,file="${isos[0]}" \
  -device ide-cd,drive=iso,bootindex=2 \
  -nic user,model=virtio-net-pci \
  -vga none \
  -device virtio-gpu-pci \
  -device qemu-xhci \
  -device usb-tablet
