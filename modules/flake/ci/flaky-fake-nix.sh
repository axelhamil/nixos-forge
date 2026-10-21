fail_build() {
  echo "error: builder for '/nix/store/x-$check.drv' failed with exit code 1" >&2
  exit 1
}

if [ "$*" = "eval --json .#ci.flakyMatrix" ]; then
  echo "$*" >>"$STATE/calls"
  cat "$MATRIX_FILE"
  exit 0
fi

[ "$1" = build ] || {
  echo "unexpected nix call: $*" >&2
  exit 1
}

installable=${*: -1}
check=${installable##*.}
echo "$*" >>"$STATE/calls"

if [[ " $* " == *" --rebuild "* ]]; then
  if [ ! -e "$STATE/$check.realised" ]; then
    echo "$check" >>"$STATE/rebuilt-before-realise"
    echo "error: some outputs of '/nix/store/x-$check.drv' are not valid, so checking is not possible" >&2
    exit 1
  fi

  count=$(($(cat "$STATE/$check.rebuilds" 2>/dev/null || echo 0) + 1))
  echo "$count" >"$STATE/$check.rebuilds"

  case "$check" in
  flaky) [ "$count" -ne 3 ] || fail_build ;;
  nondeterministic)
    echo "error: derivation '/nix/store/x-$check.drv' may not be deterministic: output \"/nix/store/x-$check\" differs" >&2
    exit 1
    ;;
  esac
  exit 0
fi

case "$check" in
broken) fail_build ;;
late)
  if [ ! -e "$STATE/late.tried" ]; then
    touch "$STATE/late.tried"
    fail_build
  fi
  ;;
esac

touch "$STATE/$check.realised"
