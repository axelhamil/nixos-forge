if ! volume_key=$(keyctl search @u user "$volume_key_description"); then
  echo "No volume key of $device linked by the unlock"
  exit 0
fi

header=$(cryptsetup luksDump --dump-json-metadata "$device")

if jq -e 'any(.tokens[]; .type == "systemd-tpm2")' <<<"$header" >/dev/null; then
  keyctl invalidate "$volume_key"
  echo "$device already has a TPM2 token, dropped its volume key before leaving the initrd"
  exit 0
fi

keyctl timeout "$volume_key" "$timeout"
echo "Kept the volume key of $device for tpm2-enroll, expiring in $timeout seconds"
