state_dir=$1
mkdir -p "$state_dir/swtpm"

cp --no-preserve=mode "$FIRMWARE_VARS" "$state_dir/vars.fd"

swtpm socket \
  --tpmstate dir="$state_dir/swtpm" \
  --ctrl type=unixio,path="$state_dir/swtpm/socket.ctrl" \
  --tpm2 \
  --daemon

until [ -S "$state_dir/swtpm/socket.ctrl" ]; do
  sleep 0.1
done
