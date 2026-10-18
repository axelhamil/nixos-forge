usage="usage: reproducibility [<host>...]"

case "${1:-}" in
-h | --help)
  echo "$usage"
  exit 0
  ;;
-*)
  echo "$usage" >&2
  exit 2
  ;;
esac

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

if [ $# -gt 0 ]; then
  hosts=("$@")
else
  mapfile -t hosts < <(nix eval --impure --json .#ci.hosts --apply 'hosts: hosts.${builtins.currentSystem}' | jq -r '.[]')
fi

echo "## Reproducibility"
echo
echo "Each system is built, then built again with \`nix build --rebuild\` and compared. A difference is expected now and then and fails nothing."
echo

status=0
for host in "${hosts[@]}"; do
  installable=".#nixosConfigurations.$host.config.system.build.toplevel"

  if ! path=$(nix build --no-link --print-out-paths "$installable" 2>"$work/build.log"); then
    echo "- \`$host\`: the system did not build, see the job log."
    tail -n 20 "$work/build.log" >&2
    status=1
    continue
  fi

  rm -rf "$path.check" 2>/dev/null || true
  if [ -e "$path.check" ]; then
    echo "- \`$host\`: \`$path.check\` is left from an earlier run and cannot be removed, so a difference could not be told apart; remove it as root and run again."
    status=1
    continue
  fi

  if nix build --no-link --rebuild --keep-failed "$installable" 2>"$work/rebuild.log"; then
    echo "- \`$host\`: the rebuild is identical."
    continue
  fi

  if [ ! -e "$path.check" ]; then
    echo "- \`$host\`: the rebuild did not finish, see the job log."
    tail -n 20 "$work/rebuild.log" >&2
    status=1
    continue
  fi

  diffoscope --text "$work/$host.diff" --exclude-directory-metadata=yes "$path" "$path.check" >/dev/null || true
  echo "- \`$host\`: the rebuild differs from the first build."
  echo
  echo "<details>"
  echo "<summary>diffoscope of $host, first 200 lines</summary>"
  echo
  echo '```'
  head -n 200 "$work/$host.diff"
  echo '```'
  echo "</details>"
  echo
done

exit "$status"
