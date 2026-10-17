host_of() {
  sed -E 's/.*nixosConfigurations\.([^.]+)\..*/\1/' <<<"$1"
}

case "$1" in
eval)
  echo '["same", "differs"]'
  ;;
build)
  installable=${*: -1}
  host=$(host_of "$installable")
  path=$STORE/$host-system

  if [ "$host" = broken ]; then
    echo "error: builder for '$host' failed" >&2
    exit 1
  fi

  if [[ " $* " != *" --rebuild "* ]]; then
    mkdir -p "$path"
    echo "built at 1" >"$path/version"
    echo "$path"
    exit 0
  fi

  [ "$host" = differs ] || exit 0

  mkdir -p "$path.check"
  echo "built at 2" >"$path.check/version"
  echo "error: derivation '$host.drv' may not be deterministic: output '$path' differs from '$path.check'" >&2
  exit 1
  ;;
*)
  echo "unexpected nix call: $*" >&2
  exit 1
  ;;
esac
