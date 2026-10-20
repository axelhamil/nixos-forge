root=$(realpath "${1:-.}")
cd "$root"

pending_file=modules/flake/docs/pending.txt
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

declare -A found=()

record() {
  local key=$1 message=$2
  found[$key]="${found[$key]:+${found[$key]}
}$message"
}

status=0
lychee --offline --include-fragments --no-progress --format json README.md docs >"$work/links.json" || status=$?
if ((status != 0 && status != 2)); then
  echo "check-docs: lychee failed with status $status" >&2
  exit 1
fi

while IFS=$'\t' read -r file target line text; do
  record "link $file $target" "$file:$line: broken link to $target ($text)"
done < <(jq -r --arg prefix "file://$root/" '
  .error_map | to_entries[] | .key as $file | .value[]
  | [$file, (.url | ltrimstr($prefix)), (.span.line // 0 | tostring), .status.text] | @tsv
' "$work/links.json")

if [[ -f docs/README.md ]]; then
  { rg -o --no-filename -r '$1' '\]\(([^)#[:space:]]+)' docs/README.md || true; } |
    while IFS= read -r target; do
      realpath -m --relative-to=. "docs/$target"
    done | sort -u >"$work/indexed"

  while IFS= read -r page; do
    grep -qxF "$page" "$work/indexed" || record "orphan $page" "$page: no link to it in docs/README.md, the docs index"
  done < <(find docs -name '*.md' ! -path docs/README.md | sort)
fi

fido2_pam_origin="pam://nixos-forge"

forbidden_names=(alice example-server example-pi axelhamil nixos-forge nixos-starter example "the starter")
forbidden_patterns=(
  '\balice\b'
  '\bexample-server\b'
  '\bexample-pi\b'
  'axelhamil'
  'nixos-forge'
  'nixos-starter'
  '\x60example\x60|(?<![\w-])example(?= hosts?\b)|(?<![\w-])example-(?:disk|boots|toplevel|installer|wipe-home)(?![\w-])|deploy/example(?![\w-])'
  '\bthe starter\b'
)

while IFS= read -r file; do
  for index in "${!forbidden_names[@]}"; do
    name=${forbidden_names[$index]}
    lines=$(sed "s|$fido2_pam_origin||g" "$file" |
      rg -n -i -P --no-heading "${forbidden_patterns[$index]}" |
      grep -vF -- "<!-- docs-check: allow $name -->" |
      cut -d: -f1 | paste -sd, || true)
    [[ -z $lines ]] || record "forbidden $file $name" "$file:$lines: names \"$name\", which a fork does not have (or mark the line with <!-- docs-check: allow $name -->)"
  done
done < <(find docs -name '*.md' ! -path 'docs/maintenance/*' | sort)

rg --no-filename -N '^#{1,6} ' -g '*.md' docs >"$work/headings" || true
rg --no-filename -N '^\|' -g '*.md' docs >"$work/rows" || true

modules_of() {
  find "$1" -mindepth 1 -maxdepth 1 -type d -printf '%f\n' | sort
}

covered() {
  local name=$1
  rg -q -F -- "\`$name\`" "$work/headings" ||
    rg -q -- "^\\|[^|]*\`$name\`[^|]*\\|" "$work/rows"
}

check_kind() {
  local kind=$1 dir=$2 name
  while IFS= read -r name; do
    case "$kind $name" in
    "feature base" | "feature profiles" | "feature hardware") continue ;;
    esac
    covered "$name" || record "section $kind $name" "docs/: no heading or first table cell names the $kind \`$name\` ($dir/$name)"
  done < <(modules_of "$dir")
}

check_kind app modules/apps
check_kind feature modules/features
check_kind hardware modules/features/hardware
check_kind profile modules/features/profiles

declare -A pending=()
if [[ -f $pending_file ]]; then
  while IFS= read -r entry; do
    [[ -n $entry ]] && pending[$entry]=1
  done <"$pending_file"
fi

problems=0
while IFS= read -r key; do
  [[ -n $key ]] || continue
  [[ -n ${pending[$key]:-} ]] && continue
  printf '%s [%s]\n' "${found[$key]}" "$key" >&2
  problems=$((problems + 1))
done < <(printf '%s\n' "${!found[@]}" | sort)

for entry in "${!pending[@]}"; do
  [[ -n ${found[$entry]:-} ]] && continue
  echo "$pending_file: \"$entry\" no longer fails: remove its line" >&2
  problems=$((problems + 1))
done

if ((problems > 0)); then
  echo "check-docs: $problems problem(s); a known violation waiting for its fix goes in $pending_file as \"<rule> <what>\"" >&2
  exit 1
fi

echo "check-docs: docs pass, ${#pending[@]} pending entries in $pending_file"
