template_dir=modules/hosts/example
template=$template_dir/default.nix
example_person=alice
person_template=modules/users/$example_person/default.nix
upstream=${UPSTREAM:-axelhamil/nixos-forge}
profiles=(desktop laptop server pi)
systems=(x86_64-linux aarch64-linux)

usage() {
  cat <<'EOF'
Usage: nix run .#init -- [options]

Writes modules/hosts/<host>/default.nix from the commented example host and
modules/users/<login>/default.nix from the example person, removes the examples and stages
the result. Asks for every answer not given as an option.

  --host NAME            host name: 1 to 63 letters, digits, - and _, starting and ending with a letter or digit
  --system SYSTEM        x86_64-linux (default) or aarch64-linux
  --profile PROFILE      desktop, laptop, server or pi
  --login LOGIN          your login name, and the name of your person in modules/users
  --full-name NAME       your full name
  --email EMAIL          your email, used by git; empty for none
  --time-zone ZONE       such as Europe/Paris (default: this machine's)
  --keyboard LAYOUT      such as fr (default: us)
  --locale LOCALE        such as fr_FR.UTF-8 (default: en_US.UTF-8)
  --features LIST        features to turn on, comma or space separated (default: picked for the profile)
  --hardware LIST        hardware modules or nixos-hardware profiles (default: firmware audio bluetooth for desktop and laptop, raspberry-pi-5 for pi, else none)
  --apps LIST            apps for every user: starter apps or nixpkgs packages (default: picked for the profile)
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
apps="" apps_set=false
keep_server="" keep_pi="" yes=false

parse_args() {
  local option value has_value
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
    --host | --system | --profile | --login | --full-name | --email | --time-zone | --keyboard | --locale | --features | --hardware | --apps)
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
    --apps) apps=$value apps_set=true ;;
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
}

enter_checkout() {
  local root
  root=$(git rev-parse --show-toplevel 2>/dev/null) || die "run it inside a git checkout of your fork"
  cd "$root"

  [[ -f $template ]] || die "$template is missing: init starts from the example host, which a previous run already replaced"
  [[ -f $person_template ]] || die "$person_template is missing: init starts from the example person, which a previous run already replaced"
}

list_block() {
  awk -v block="$1" '
    $0 ~ "^ *" block " = \\[$" { inside = 1; next }
    inside && /^ *\];/ { exit }
    inside && match($0, /^ *(# )?"([^"]+)"( # (.*))?$/, m) { print m[2] "\t" m[4] }
  ' "$template"
}

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
  [[ $1 == "$example_person" || ! -e modules/users/$1 ]] || {
    echo "modules/users/$1 already exists: pick another login, init never overwrites a person"
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

check_names() {
  local item
  for item in $(words "$1"); do
    check_word "$item" >/dev/null || {
      echo "\"$item\" is not a module or package name"
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
  desktop | laptop) wanted=(disk plymouth greetd nix-ld printing) ;;
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

default_hardware() {
  case $1 in
  desktop | laptop) printf 'firmware audio bluetooth' ;;
  pi) printf raspberry-pi-5 ;;
  esac
}

default_apps() {
  case $1 in
  desktop | laptop) printf 'hyprland waybar fuzzel kitty mako lock upgrade-notify direnv firefox' ;;
  esac
}

show_list() {
  local block=$1 title=$2 chosen name description mark
  mapfile -t chosen < <(words "$3")
  echo "$title (each one is described in $template):" >&2
  while IFS=$'\t' read -r name description; do
    mark=" "
    if contains "$name" "${chosen[@]}"; then
      mark=x
    fi
    printf '  [%s] %-14s %s\n' "$mark" "$name" "$description" >&2
  done < <(list_block "$block")
}

ask_email() {
  if [[ $email_set == true ]]; then
    ask email email "" "" check_email set
    return
  fi
  if [[ $yes == true ]]; then
    die "--email is required with --yes (--email '' for none)"
  fi
  ask email email "Email, for git (blank for none)" "" check_email true
}

ask_list() {
  local name=$1 given=$2 suggested=$3 check=$4 prompt=$5 title=${6:-}
  if [[ $given == true ]]; then
    ask "$name" "$name" "" "" "$check" set
    return
  fi
  if [[ $yes == true ]]; then
    printf -v "$name" '%s' "$suggested"
    return
  fi
  if [[ -n $title ]]; then
    show_list "$name" "$title" "$suggested"
  fi
  ask "$name" "$name" "$prompt" "$suggested" "$check" true
}

collect_answers() {
  local default_profile=desktop
  ask host host "Host name" "" check_host
  ask system system "System (${systems[*]})" x86_64-linux check_system
  [[ $system == aarch64-linux ]] && default_profile=pi
  ask profile profile "Profile (${profiles[*]})" "$default_profile" check_profile
  ask login login "Login" "" check_login
  ask full_name full-name "Full name" "" check_text
  ask_email

  ask time_zone time-zone "Time zone" "$(detected_time_zone)" check_word
  ask keyboard keyboard "Keyboard layout" us check_word
  ask locale locale "Locale" "$(detected_locale)" check_word

  ask_list features "$features_set" "$(default_features "$profile" "$system")" check_features \
    "Features, space separated (blank keeps the ones marked)" Features
  ask_list hardware "$hardware_set" "$(default_hardware "$profile")" check_names \
    "Hardware: starter modules or nixos-hardware profiles, space separated (blank for none)"
  ask_list apps "$apps_set" "$(default_apps "$profile")" check_names \
    "Apps for every user: starter apps or nixpkgs packages, space separated (blank keeps the ones marked)" Apps

  if [[ -d modules/hosts/example-server ]]; then
    ask_yes_no keep_server "Keep the example-server host"
  fi
  if [[ -d modules/hosts/example-pi ]]; then
    ask_yes_no keep_pi "Keep the example-pi host"
  fi
}

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

cleanup() {
  local status=$?
  rm -f "$generated" "$generated_person" "$key_file.pub"
  if ((status != 0)) && [[ -f $key_file ]]; then
    echo "init: the deploy signing key is kept in $key_file, its public key may already be in the signers of your hosts" >&2
  fi
}

make_deploy_key() {
  local key_dir key_type key_body
  if [[ -d ${XDG_RUNTIME_DIR:-} ]]; then
    key_dir=$(mktemp -d -p "$XDG_RUNTIME_DIR" deploy-key.XXXXXX)
  else
    key_dir=$(mktemp -d)
  fi
  key_file=$key_dir/deploy
  trap cleanup EXIT

  ssh-keygen -q -t ed25519 -N "" -C deploy -f "$key_file" </dev/null
  read -r key_type key_body _ <"$key_file.pub"
  rm -f -- "$key_file.pub"
  signer="$key_type $key_body deploy"
}

trust_signer() {
  SIGNER=$signer awk '
    match($0, /^    # signers = \[ "[^"]*" \];(.*)$/, m) {
      print "    signers = ["
      print "      \"" ENVIRON["SIGNER"] "\""
      print "    ];" m[1]
      next
    }

    { print }
  '
}

render_host() {
  HOST_ATTRIBUTE=$(nix_attribute "$host") \
    SYSTEM=$system \
    PROFILE=$profile \
    LOGIN=$login \
    LOGIN_ATTRIBUTE=$login_attribute \
    TIME_ZONE=$time_zone \
    KEYBOARD=$keyboard \
    LOCALE=$locale \
    FEATURES=$(words "$features" | tr '\n' ' ') \
    HARDWARE=$(words "$hardware" | tr '\n' ' ') \
    APPS=$(words "$apps" | tr '\n' ' ') \
    awk -f "$SET_STRING" --source '
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
      /^    # mirrors = / { gsub(/alice/, ENVIRON["LOGIN"]); print; next }
      /^    timeZone = "/ { print set_string($0, ENVIRON["TIME_ZONE"]); next }
      /^    locale = "/ { print set_string($0, ENVIRON["LOCALE"]); next }
      /^    keyboard = "/ { print set_string($0, ENVIRON["KEYBOARD"]); next }
      /^    users\.alice = \{$/ { sub(/alice/, ENVIRON["LOGIN_ATTRIBUTE"]); print; next }

      { print }
    ' "$template" | trust_signer
}

render_person() {
  LOGIN=$login \
    LOGIN_ATTRIBUTE=$login_attribute \
    FULL_NAME=$(nix_string "$full_name") \
    EMAIL=$(nix_string "$email") \
    awk -f "$SET_STRING" --source '
      /^  users\.alice = \{$/ { sub(/alice/, ENVIRON["LOGIN_ATTRIBUTE"]); print; next }
      /^    fullName = "/ { print set_string($0, ENVIRON["FULL_NAME"]); next }
      /^    email = "/ {
        if (ENVIRON["EMAIL"] == "") { sub(/email/, "# email"); gsub(/alice/, ENVIRON["LOGIN"]); print }
        else print set_string($0, ENVIRON["EMAIL"])
        next
      }

      { gsub(/users\.alice/, "users." ENVIRON["LOGIN_ATTRIBUTE"]); gsub(/alice/, ENVIRON["LOGIN"]); print }
    ' "$person_template"
}

write_host_and_person() {
  render_host >"$generated"
  render_person >"$generated_person"

  install -D -m 644 "$generated" "$target"
  install -D -m 644 "$generated_person" "$person"

  if command -v nix >/dev/null; then
    nix fmt -- "$target" "$person" >/dev/null 2>&1 || echo "init: nix fmt failed, run it yourself" >&2
  fi

  git add -- "$target" "$person"
}

remove_examples() {
  local removed=(example) example
  [[ $keep_server == yes ]] || removed+=(example-server)
  [[ $keep_pi == yes ]] || removed+=(example-pi)
  for example in "${removed[@]}"; do
    [[ $example == "$host" || ! -d modules/hosts/$example ]] && continue
    git rm -r -q -- "modules/hosts/$example"
  done

  if [[ $login != "$example_person" ]]; then
    git rm -r -q -- "modules/users/$example_person"
  fi
}

github_repo() {
  local origin repo
  origin=$(git remote get-url origin 2>/dev/null) || return 0
  repo=${origin%/}
  repo=${repo%.git}
  if [[ $repo =~ ^(https://github\.com/|ssh://git@github\.com/|git@github\.com:)([^/]+/[^/]+)$ ]]; then
    printf '%s' "${BASH_REMATCH[2]}"
  fi
}

fork_flake() {
  local origin repo
  origin=$(git remote get-url origin 2>/dev/null) || return 0
  repo=$(github_repo)
  if [[ -n $repo ]]; then
    [[ ${repo,,} == "${upstream,,}" ]] || printf 'github:%s' "$repo"
  elif [[ $origin =~ ^https://[^?#]+$ ]]; then
    printf 'git+%s' "$origin"
  fi
}

update_kept_example() {
  local example=$1 kept_flake=$2
  if [[ $login != "$example_person" ]]; then
    sed -i "s/^    users\.$example_person = {\$/    users.$login_attribute = {/" "$example/default.nix"
  fi
  KEPT_FLAKE=$kept_flake awk -f "$SET_STRING" --source '
    /^    (# )?flake = "github:/ {
      if (ENVIRON["KEPT_FLAKE"] != "") print set_string($0, ENVIRON["KEPT_FLAKE"])
      else { sub(/^    (# )?/, "    # "); print }
      next
    }

    ENVIRON["KEPT_FLAKE"] == "" && /^      "auto-upgrade" #/ { sub(/"auto-upgrade"/, "# \"auto-upgrade\""); print; next }

    { print }
  ' "$example/default.nix" | trust_signer >"$generated"
  cat -- "$generated" >"$example/default.nix"
  git add -- "$example/default.nix"
}

update_kept_examples() {
  local kept_flake example
  kept_flake=$(fork_flake)
  for example in modules/hosts/example-server modules/hosts/example-pi; do
    [[ -f $example/default.nix && $example != "$target_dir" ]] || continue
    update_kept_example "$example" "$kept_flake"
  done
}

point_readme() {
  [[ -f README.md ]] || return 0
  HOST=$host TARGET=$target PERSON=$person awk '
    /^The commented \[`example`\]\(modules\/hosts\/example\/default\.nix\) host / {
      print "Your [`" ENVIRON["HOST"] "`](" ENVIRON["TARGET"] ") host lists every profile, feature, hardware module and app upstream shipped when you ran `nix run .#init`, one line each, commented out when you did not pick it. Read it top to bottom and keep what you like."
      next
    }

    /^   - writes `modules\/hosts\/<host>\/default\.nix` from the \[example host\]/ {
      print "   - writes your host, [`" ENVIRON["TARGET"] "`](" ENVIRON["TARGET"] "), from the example host and your person, [`" ENVIRON["PERSON"] "`](" ENVIRON["PERSON"] "), from the example person;"
      next
    }

    { print }
  ' README.md >"$generated"
  cat -- "$generated" >README.md
  git add -- README.md
}

print_next_steps() {
  local repo quoted_key
  cat <<EOF

$target and $person are ready and staged; the example hosts you did not
keep are removed. Read them top to bottom: every line you may want is there, commented out.

Next steps:
  nix develop          installs the pre-push hook
  nix flake check -L   builds every host and boots it in a VM
  nix build .#nixosConfigurations.$host.config.system.build.vm && ./result/bin/run-$host-vm
                       boots $host in a VM: log in as $login with the password "test"
  nix build .#installer-$host
                       the installer ISO, for a host with the disk feature (docs/tutorials/install.md)

Deploy signing key: CI signs every deploy/<host> commit with it, and auto-upgrade
applies only commits signed by its public key, written in the signers of your hosts.
The private key is in $key_file, outside the repository and readable
by you alone. In your fork, create the environment deploy limited to the main branch
(Settings, Environments; docs/how-to/ci.md), then store the key there and delete the file:

EOF

  repo=$(github_repo)
  printf -v quoted_key '%q' "$key_file"
  if [[ -n $repo && ${repo,,} != "${upstream,,}" ]]; then
    echo "  gh secret set DEPLOY_SIGNING_KEY --env deploy --repo $repo < $quoted_key && shred -u $quoted_key"
  else
    echo "  gh secret set DEPLOY_SIGNING_KEY --env deploy < $quoted_key && shred -u $quoted_key"
  fi
  cat <<EOF

or, on GitHub, paste the content of the file in the secret DEPLOY_SIGNING_KEY of the
environment deploy, then run: shred -u $quoted_key
EOF
}

parse_args "$@"
enter_checkout
mapfile -t known_features < <(list_block features | cut -f1)
collect_answers

target_dir=modules/hosts/$host
target=$target_dir/default.nix
person=modules/users/$login/default.nix
login_attribute=$(nix_attribute "$login")
generated=$(mktemp)
generated_person=$(mktemp)
key_file=""
make_deploy_key

write_host_and_person
remove_examples
update_kept_examples
point_readme
print_next_steps
