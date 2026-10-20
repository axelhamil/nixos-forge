file="$directory/$(date +%Y-%m-%d_%H-%M-%S).png"
mkdir -p "$directory"

case "${1:-area}" in
  area) grim -g "$(slurp)" "$file" ;;
  screen) grim "$file" ;;
  *)
    echo "usage: screenshot [area|screen]" >&2
    exit 2
    ;;
esac

wl-copy <"$file"
notify-send --app-name screenshot "Screenshot copied" "$file" || true
