device=${1:-$default_device}

if [ -z "$device" ]; then
  echo "Usage: fido2-enroll <LUKS device>, this host has no disk feature to default to" >&2
  exit 1
fi

systemd-cryptenroll --fido2-device=auto "$device"

echo "Enrolled a security key on $device, the passphrase stays as a fallback"
