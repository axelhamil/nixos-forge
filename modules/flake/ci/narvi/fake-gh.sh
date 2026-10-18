echo "$*" >>"$GH_STATE/calls"

if [ -e "$GH_STATE/fail" ]; then
  echo "gh: HTTP 502: Bad Gateway" >&2
  exit 1
fi

if [ -e "$GH_STATE/fail-edit" ] && [ "$2" = edit ]; then
  echo "gh: HTTP 403: Resource not accessible by integration" >&2
  exit 1
fi

case "$1 $2" in
"pr view" | "release view")
  cat "$GH_STATE/$1-$3.md"
  ;;
"pr edit")
  [ "$4" = --body-file ] || exit 2
  cp "$5" "$GH_STATE/pr-$3.md"
  ;;
"release edit")
  [ "$4" = --notes-file ] || exit 2
  cp "$5" "$GH_STATE/release-$3.md"
  ;;
*)
  echo "fake gh: unexpected call $*" >&2
  exit 2
  ;;
esac
