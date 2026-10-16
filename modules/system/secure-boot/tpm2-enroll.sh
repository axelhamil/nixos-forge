temporary_slot=31

efi_variable() {
  od -An -tu1 -j4 -N1 "/sys/firmware/efi/efivars/$1-8be4df61-93ca-11d2-aa0d-00e098032b8c" 2>/dev/null | tr -d ' \n'
}

secure_boot_enforced() {
  [ "$(efi_variable SecureBoot)" = 1 ] && [ "$(efi_variable SetupMode)" = 0 ]
}

header=$(cryptsetup luksDump --dump-json-metadata "$device")

if jq -e --arg slot "$temporary_slot" '
  (.keyslots | has($slot)) and (.keyslots | length > 1) and all(.tokens[]; .keyslots | any(. == $slot) | not)
' <<<"$header" >/dev/null; then
  systemd-cryptenroll --wipe-slot="$temporary_slot" "$device"
  echo "Wiped the temporary keyslot $temporary_slot left by an interrupted enrollment"
  header=$(cryptsetup luksDump --dump-json-metadata "$device")
fi

if ! volume_key=$(keyctl search @u user "$volume_key_description"); then
  echo "No volume key of $device linked at this boot, nothing to enroll"
  exit 0
fi

unlock_key="$RUNTIME_DIRECTORY/unlock-key"
temporary_key_added=false

cleanup() {
  failed=0

  keyctl invalidate "$volume_key" || failed=1

  if [ "$temporary_key_added" = true ]; then
    if systemd-cryptenroll --wipe-slot="$temporary_slot" "$device"; then
      echo "Removed the temporary keyslot $temporary_slot"
    else
      failed=1
    fi
  fi

  rm -f "$unlock_key" || failed=1

  if [ "$failed" = 1 ]; then
    echo "Cleanup failed, check the keyslots of $device"
    exit 1
  fi
}
trap cleanup EXIT
trap 'exit 143' TERM INT HUP

if jq -e 'any(.tokens[]; .type == "systemd-tpm2")' <<<"$header" >/dev/null; then
  echo "$device already has a TPM2 token, nothing to enroll"
  exit 0
fi

if [ ! -c /dev/tpmrm0 ]; then
  echo "No TPM2 on this machine, the passphrase stays the only way to unlock $device"
  exit 0
fi

if ! secure_boot_enforced; then
  echo "Secure Boot is not enforced yet, TPM2 enrollment of $device waits for a boot in user mode"
  exit 0
fi

head -c 48 /dev/urandom | base64 -w0 >"$unlock_key"

cryptsetup luksAddKey \
  --batch-mode \
  --volume-key-keyring "%user:$volume_key_description" \
  --new-key-slot "$temporary_slot" \
  --pbkdf pbkdf2 \
  --pbkdf-force-iterations 1000 \
  "$device" "$unlock_key"
temporary_key_added=true

systemd-cryptenroll \
  --unlock-key-file="$unlock_key" \
  --tpm2-device=auto \
  --tpm2-pcrs="$pcrs" \
  "$device"

echo "Enrolled a TPM2 token on $device bound to PCRs $pcrs, the passphrase stays as a fallback"
