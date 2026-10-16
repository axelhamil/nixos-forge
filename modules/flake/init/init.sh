template_dir=modules/hosts/example
template=$template_dir/default.nix
profiles=(desktop laptop server pi)
systems=(x86_64-linux aarch64-linux)

usage() {
  cat <<'EOF'
Usage: nix run .#init -- [options]

Writes modules/hosts/<host>/default.nix from the commented example host, removes the
example hosts and stages the result. Asks for every answer not given as an option.

  --host NAME            host name: 1 to 63 letters, digits, - and _, starting and ending with a letter or digit
  --system SYSTEM        x86_64-linux (default) or aarch64-linux
  --profile PROFILE      desktop, laptop, server or pi
  --login LOGIN          your login name
  --full-name NAME       your full name
  --email EMAIL          your email, used by git; empty for none
  --time-zone ZONE       such as Europe/Paris (default: this machine's)
  --keyboard LAYOUT      such as fr (default: us)
  --locale LOCALE        such as fr_FR.UTF-8 (default: en_US.UTF-8)
  --features LIST        features to turn on, comma or space separated (default: picked for the profile)
  --hardware LIST        hardware modules or nixos-hardware profiles (default: raspberry-pi-5 for pi, else none)
  --keep-server          keep the example-server host
  --keep-pi              keep the example-pi host
  --yes                  ask nothing: use the defaults, fail on a missing answer
  -h, --help             show this help
EOF
}

die() {
  printf 'init: %s\n' "$*" >&2
  exit 1
}

host="" system="" profile="" login="" full_name="" time_zone="" keyboard="" locale=""
email="" email_set=false
features="" features_set=false
hardware="" hardware_set=false
keep_server="" keep_pi="" yes=false

while (($# > 0)); do
  option=$1
  value=""
  has_value=false
  if [[ $option == --*=* ]]; then
    value=${option#*=}
    option=${option%%=*}
    has_value=true
  fi
  shift

  case $option in
  --host | --system | --profile | --login | --full-name | --email | --time-zone | --keyboard | --locale | --features | --hardware)
    if [[ $has_value == false ]]; then
      (($# > 0)) || die "$option needs a value"
      value=$1
      shift
    fi
    ;;
  esac

  case $option in
  --host) host=$value ;;
  --system) system=$value ;;
  --profile) profile=$value ;;
  --login) login=$value ;;
  --full-name) full_name=$value ;;
  --email) email=$value email_set=true ;;
  --time-zone) time_zone=$value ;;
  --keyboard) keyboard=$value ;;
  --locale) locale=$value ;;
  --features) features=$value features_set=true ;;
  --hardware) hardware=$value hardware_set=true ;;
  --keep-server) keep_server=yes ;;
  --keep-pi) keep_pi=yes ;;
  --yes) yes=true ;;
  -h | --help)
    usage
    exit 0
    ;;
  *) die "unknown option $option (see --help)" ;;
  esac
done

root=$(git rev-parse --show-toplevel 2>/dev/null) || die "run it inside a git checkout of your fork"
cd "$root"

[[ -f $template ]] || die "$template is missing: init starts from the example host, which a previous run already replaced"

list_block() {
  awk -v block="$1" '
    $0 ~ "^ *" block " = \\[$" { inside = 1; next }
    inside && /^ *\];/ { exit }
    inside && match($0, /^ *(# )?"([^"]+)"( # (.*))?$/, m) { print m[2] "\t" m[4] }
  ' "$template"
}

mapfile -t known_features < <(list_block features | cut -f1)

contains() {
  local needle=$1 item
  shift
  for item in "$@"; do
    [[ $item == "$needle" ]] && return 0
  done
  return 1
}

words() {
  tr -s ', ' '\n' <<<"$1" | awk 'NF && !seen[$0]++'
}

check_host() {
  [[ $1 =~ ^[A-Za-z0-9]([A-Za-z0-9_-]{0,61}[A-Za-z0-9])?$ ]] || {
    echo "a host name has 1 to 63 letters, digits, - and _, and starts and ends with a letter or digit"
    return 1
  }
  [[ ! -e modules/hosts/$1 ]] || {
    echo "modules/hosts/$1 already exists: pick another host name, init never overwrites a host"
    return 1
  }
}

check_system() {
  contains "$1" "${systems[@]}" || {
    echo "the system is one of: ${systems[*]}"
    return 1
  }
}

check_profile() {
  contains "$1" "${profiles[@]}" || {
    echo "the profile is one of: ${profiles[*]}"
    return 1
  }
}

check_login() {
  [[ $1 =~ ^[a-z_][a-z0-9_-]{0,31}$ ]] || {
    echo "a login has at most 32 lowercase letters, digits, - and _, and starts with a letter or _"
    return 1
  }
}

check_text() {
  [[ -n $1 && $1 != *[[:cntrl:]]* ]] || {
    echo "a non-empty value on one line"
    return 1
  }
}

check_email() {
  [[ -z $1 || $1 =~ ^[^[:space:][:cntrl:]\"]+@[^[:space:][:cntrl:]\"]+$ ]] || {
    echo "an email such as alice@example.org, or nothing"
    return 1
  }
}

check_word() {
  [[ $1 =~ ^[A-Za-z0-9_+./-]+$ ]] || {
    echo "letters, digits and _ + . / - only"
    return 1
  }
}

check_features() {
  local feature
  for feature in $(words "$1"); do
    contains "$feature" "${known_features[@]}" || {
      echo "unknown feature \"$feature\"; known: ${known_features[*]}"
      return 1
    }
  done
}

check_hardware() {
  local item
  for item in $(words "$1"); do
    check_word "$item" >/dev/null || {
      echo "\"$item\" is not a module name"
      return 1
    }
  done
}

ask() {
  local name=$1 flag=$2 prompt=$3 default=$4 check=$5 optional=${6:-false}
  local value=${!name} reply error

  if [[ -n $value || $optional == set ]]; then
    error=$("$check" "$value") || die "--$flag: $error"
    return 0
  fi

  if [[ $yes == true ]]; then
    [[ -n $default || $optional == true ]] || die "--$flag is required with --yes"
    error=$("$check" "$default") || die "--$flag: $error"
    printf -v "$name" '%s' "$default"
    return 0
  fi

  while true; do
    if [[ -n $default ]]; then
      read -r -p "$prompt [$default]: " reply || die "no answer for $prompt: pass --$flag, or --yes for the defaults"
    else
      read -r -p "$prompt: " reply || die "no answer for $prompt: pass --$flag, or --yes for the defaults"
    fi
    reply=${reply:-$default}
    if [[ -z $reply && $optional != true ]]; then
      echo "  an answer is needed" >&2
      continue
    fi
    if error=$("$check" "$reply"); then
      printf -v "$name" '%s' "$reply"
      return 0
    fi
    echo "  $error" >&2
  done
}

ask_yes_no() {
  local name=$1 prompt=$2 reply
  [[ -n ${!name} ]] && return 0
  if [[ $yes == true ]]; then
    printf -v "$name" no
    return 0
  fi
  read -r -p "$prompt [y/N]: " reply || reply=n
  if [[ $reply == [yY]* ]]; then
    printf -v "$name" yes
  else
    printf -v "$name" no
  fi
}

detected_time_zone() {
  local zone
  zone=$(readlink -f /etc/localtime 2>/dev/null) || zone=""
  if [[ $zone == */zoneinfo/* ]]; then
    printf '%s' "${zone##*/zoneinfo/}"
  else
    printf UTC
  fi
}

detected_locale() {
  local lang=${LC_ALL:-${LANG:-}}
  lang=${lang/.utf8/.UTF-8}
  if [[ $lang =~ ^[a-z]{2,3}_[A-Z]{2}\.UTF-8$ ]]; then
    printf '%s' "$lang"
  else
    printf en_US.UTF-8
  fi
}

default_features() {
  local wanted=() feature
  case $1 in
  desktop | laptop) wanted=(disk plymouth hyprland nix-ld direnv) ;;
  server) wanted=(disk ssh) ;;
  pi) wanted=(ssh) ;;
  esac
  if [[ $2 == x86_64-linux && $1 != pi ]]; then
    wanted+=(secure-boot)
  fi
  for feature in "${known_features[@]}"; do
    if contains "$feature" "${wanted[@]}"; then
      printf '%s ' "$feature"
    fi
  done
}

show_features() {
  local chosen name description mark
  mapfile -t chosen < <(words "$1")
  echo "Features (each one is described in $template):" >&2
  while IFS=$'\t' read -r name description; do
    mark=" "
    if contains "$name" "${chosen[@]}"; then
      mark=x
    fi
    printf '  [%s] %-14s %s\n' "$mark" "$name" "$description" >&2
  done < <(list_block features)
}

ask host host "Host name" "" check_host
ask system system "System (${systems[*]})" x86_64-linux check_system
default_profile=desktop
[[ $system == aarch64-linux ]] && default_profile=pi
ask profile profile "Profile (${profiles[*]})" "$default_profile" check_profile
ask login login "Login" "" check_login
ask full_name full-name "Full name" "" check_text

if [[ $email_set == true ]]; then
  ask email email "" "" check_email set
elif [[ $yes == true ]]; then
  die "--email is required with --yes (--email '' for none)"
else
  ask email email "Email, for git (blank for none)" "" check_email true
fi

ask time_zone time-zone "Time zone" "$(detected_time_zone)" check_word
ask keyboard keyboard "Keyboard layout" us check_word
ask locale locale "Locale" "$(detected_locale)" check_word

suggested_features=$(default_features "$profile" "$system")
if [[ $features_set == true ]]; then
  ask features features "" "" check_features set
elif [[ $yes == true ]]; then
  features=$suggested_features
else
  show_features "$suggested_features"
  ask features features "Features, space separated (blank keeps the ones marked)" "$suggested_features" check_features true
fi

suggested_hardware=""
[[ $profile == pi ]] && suggested_hardware=raspberry-pi-5
if [[ $hardware_set == true ]]; then
  ask hardware hardware "" "" check_hardware set
elif [[ $yes == true ]]; then
  hardware=$suggested_hardware
else
  ask hardware hardware "Hardware: starter modules or nixos-hardware profiles, space separated (blank for none)" "$suggested_hardware" check_hardware true
fi

[[ -d modules/hosts/example-server ]] && ask_yes_no keep_server "Keep the example-server host"
[[ -d modules/hosts/example-pi ]] && ask_yes_no keep_pi "Keep the example-pi host"

apps=""
[[ $profile == desktop || $profile == laptop ]] && apps=firefox

nix_string() {
  local text=$1
  text=${text//\\/\\\\}
  text=${text//\"/\\\"}
  text=${text//"\${"/"\\\${"}
  printf '%s' "$text"
}

nix_attribute() {
  if [[ $1 =~ ^[A-Za-z_][A-Za-z0-9_-]*$ ]] && ! contains "$1" "assert" "else" "if" "in" "inherit" "let" "or" "rec" "then" "with"; then
    printf '%s' "$1"
  else
    printf '"%s"' "$1"
  fi
}

target_dir=modules/hosts/$host
target=$target_dir/default.nix
generated=$(mktemp)
trap 'rm -f "$generated"' EXIT

HOST_ATTRIBUTE=$(nix_attribute "$host") \
  SYSTEM=$system \
  PROFILE=$profile \
  LOGIN=$login \
  LOGIN_ATTRIBUTE=$(nix_attribute "$login") \
  FULL_NAME=$(nix_string "$full_name") \
  EMAIL=$(nix_string "$email") \
  TIME_ZONE=$time_zone \
  KEYBOARD=$keyboard \
  LOCALE=$locale \
  FEATURES=$(words "$features" | tr '\n' ' ') \
  HARDWARE=$(words "$hardware" | tr '\n' ' ') \
  APPS=$apps \
  awk '
    function set_string(line, value,    start, rest) {
      start = index(line, "\"")
      rest = substr(line, start + 1)
      return substr(line, 1, start) value substr(rest, index(rest, "\""))
    }

    function choose(block, list,    names, count, i) {
      order[block] = list
      count = split(list, names, " ")
      for (i = 1; i <= count; i++) wanted[block, names[i]] = 1
    }

    BEGIN {
      choose("features", ENVIRON["FEATURES"])
      choose("hardware", ENVIRON["HARDWARE"])
      choose("apps", ENVIRON["APPS"])
    }

    match($0, /^( *)(features|hardware|apps) = \[$/, m) {
      block = m[2]
      delete seen
      print
      next
    }

    block != "" && match($0, /^( *)(# )?"([^"]+)"(.*)$/, m) {
      indent = m[1]
      seen[m[3]] = 1
      print indent (((block, m[3]) in wanted) ? "" : "# ") "\"" m[3] "\"" m[4]
      next
    }

    block != "" && /^ *\];/ {
      count = split(order[block], names, " ")
      for (i = 1; i <= count; i++) if (!(names[i] in seen)) print indent "\"" names[i] "\""
      block = ""
      print
      next
    }

    /^  configurations\.nixos\.example = \{$/ { sub(/example/, ENVIRON["HOST_ATTRIBUTE"]); print; next }
    /^    system = "/ { print set_string($0, ENVIRON["SYSTEM"]); next }
    /^    profile = "/ { print set_string($0, ENVIRON["PROFILE"]); next }
    /^    flake = "\/home\/alice\// { print set_string($0, "/home/" ENVIRON["LOGIN"] "/nixos"); next }
    /^    timeZone = "/ { print set_string($0, ENVIRON["TIME_ZONE"]); next }
    /^    locale = "/ { print set_string($0, ENVIRON["LOCALE"]); next }
    /^    keyboard = "/ { print set_string($0, ENVIRON["KEYBOARD"]); next }
    /^    users\.alice = \{$/ { sub(/alice/, ENVIRON["LOGIN_ATTRIBUTE"]); print; next }
    /^      fullName = "/ { print set_string($0, ENVIRON["FULL_NAME"]); next }
    /^      email = "/ {
      if (ENVIRON["EMAIL"] == "") { sub(/email/, "# email"); print }
      else print set_string($0, ENVIRON["EMAIL"])
      next
    }

    { print }
  ' "$template" >"$generated"

mkdir -p "$target_dir"
cp "$generated" "$target"
chmod 644 "$target"

if command -v nix >/dev/null; then
  nix fmt -- "$target" >/dev/null 2>&1 || echo "init: nix fmt failed, run it yourself" >&2
fi

git add -- "$target"

removed=(example)
[[ $keep_server == yes ]] || removed+=(example-server)
[[ $keep_pi == yes ]] || removed+=(example-pi)
for example in "${removed[@]}"; do
  [[ $example == "$host" || ! -d modules/hosts/$example ]] && continue
  git rm -r -q -- "modules/hosts/$example"
done

cat <<EOF

$target is ready and staged; the example hosts you did not keep are removed.
Read it top to bottom: every line you may want is there, commented out.

Next steps:
  nix develop          installs the pre-push hook
  nix flake check -L   builds every host and boots it in a VM
  nix build .#nixosConfigurations.$host.config.system.build.vm && ./result/bin/run-$host-vm
                       boots $host in a VM: log in as $login with the password "test"
  nix build .#installer-$host
                       the installer ISO, for a host with the disk feature (docs/install.md)
EOF
