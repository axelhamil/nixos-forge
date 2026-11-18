state_dir=$1
mkdir -p "$state_dir/swtpm"

cp --no-preserve=mode --update=none "$FIRMWARE_VARS" "$state_dir/vars.fd"

swtpm socket \
  --tpmstate dir="$state_dir/swtpm" \
  --ctrl type=unixio,path="$state_dir/swtpm/socket.ctrl" \
  --tpm2 \
  --daemon

for _ in {1..100}; do
  [[ ! -S $state_dir/swtpm/socket.ctrl ]] || break
  sleep 0.1
done
if [[ ! -S $state_dir/swtpm/socket.ctrl ]]; then
  echo "prepare-secure-machine: swtpm did not open its socket within 10 seconds" >&2
  exit 1
fi
