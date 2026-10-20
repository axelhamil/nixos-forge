starter=https://github.com/axelhamil/nixos-forge.git

usage() {
  cat <<'EOF'
Usage: nix run .#upstream -- [--yes] [URL | OWNER/REPO]

Shows what the starter changed since the main branch of origin last merged it: the breaking
changes first, then its CHANGELOG.md. Then offers to merge its latest release tag into a
chore/sync-upstream-<tag> branch made from origin/main, pushes that branch to origin and
opens a pull request against main with gh when it is logged in, or prints the gh command.

The starter is the argument, else the git remote named upstream, else
https://github.com/axelhamil/nixos-forge.git. OWNER/REPO means a GitHub repository.

  --yes        prepare the branch without asking
  -h, --help   show this help
EOF
}

die() {
  printf 'upstream: %s\n' "$*" >&2
  exit 1
}

yes=false source=""
while (($# > 0)); do
  case $1 in
  --yes | -y) yes=true ;;
  -h | --help)
    usage
    exit 0
    ;;
  -*) die "unknown option $1 (see --help)" ;;
  *)
    [[ -z $source ]] || die "one starter only, got $source and $1"
    source=$1
    ;;
  esac
  shift
done

git rev-parse --git-dir >/dev/null 2>&1 || die "run it inside your fork's checkout"

if [[ -z $source ]]; then
  source=$(git remote get-url upstream 2>/dev/null || echo "$starter")
fi

url=$source
if [[ $source =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]]; then
  url=https://github.com/$source.git
fi

releases=""
if [[ $url =~ ^https://github\.com/([^/]+/[^/]+)/?$ ]]; then
  releases=https://github.com/${BASH_REMATCH[1]%.git}/releases/tag
fi

tag=$(git ls-remote --tags "$url" 'refs/tags/v[0-9]*' | { grep -v '\^{}$' || true; } | sed 's|.*refs/tags/||' | sort -V | tail -n 1) || die "cannot read the tags of $url"
if [[ -z $tag ]]; then
  echo "$url has no release yet."
  exit 0
fi

git fetch -q --no-tags "$url" "+refs/tags/$tag:refs/tags/$tag"
git fetch -q --no-tags origin "+refs/heads/main:refs/remotes/origin/main" || die "cannot fetch the main branch of origin"
base=origin/main

if git merge-base --is-ancestor "$tag" "$base"; then
  echo "Already up to date with $tag."
  exit 0
fi

branch=chore/sync-upstream-$tag
title="chore: sync with upstream $tag"

breaking=$(git log --format=%B "$base..$tag" | grep -E '^(BREAKING[ -]CHANGE:|[a-z]+(\([^)]*\))?!:)' || true)
changelog=$(git diff "$base...$tag" -- CHANGELOG.md | grep -E '^\+' | grep -vE '^\+\+\+' | sed 's/^+//' | sed '/./,$!d' || true)

notes=""
if [[ -n $breaking ]]; then
  notes+=$'Breaking changes:\n'"$breaking"$'\n\n'
fi
notes+=$'Changelog:\n'"${changelog:-No CHANGELOG.md entry.}"

echo "$url $tag is not merged in the main branch of this fork yet."
echo
echo "$notes"
echo

if git ls-remote --exit-code --heads origin "$branch" >/dev/null; then
  echo "$branch is already on origin: its pull request is waiting for your review."
  exit 0
fi

if git rev-parse --verify -q "refs/heads/$branch" >/dev/null; then
  die "$branch already exists here: push it, or delete it with git branch -D $branch to prepare it again"
fi

if [[ -n $(git status --porcelain) ]]; then
  die "commit or stash your changes first"
fi

if ! $yes; then
  printf 'Prepare branch %s? [y/N] ' "$branch"
  answer=""
  read -r answer || true
  if [[ ! $answer =~ ^[Yy]([Ee][Ss])?$ ]]; then
    echo "Nothing changed."
    exit 0
  fi
fi

if [[ ${GITHUB_ACTIONS:-} == true ]]; then
  git config user.name "github-actions[bot]"
  git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
fi

start=$(git symbolic-ref -q --short HEAD || git rev-parse HEAD)

abandon() {
  if git rev-parse -q --verify MERGE_HEAD >/dev/null; then
    git merge --abort
  fi
  if git show-ref -q --verify "refs/heads/$start"; then
    git switch -q "$start"
  else
    git switch -q --detach "$start"
  fi
  git branch -q -D "$branch"
  die "$*"
}

git switch -q -c "$branch" "$base"
if ! merged=$(git merge --no-ff --no-edit -m "$title" "$tag" 2>&1); then
  conflicts=$(git diff --name-only --diff-filter=U | paste -sd " " -)
  if [[ -z $conflicts ]]; then
    abandon "cannot merge $tag into $base: $merged"
  fi
  abandon "$tag conflicts with the main branch of this fork in: $conflicts. Merge it by hand: git switch -c $branch $base && git merge --no-ff $tag, resolve the conflicts and commit, then git push -u origin $branch and open its pull request against main"
fi

repo=$(git remote get-url origin)
repo=${repo%/}
repo=${repo%.git}
name=${repo##*[/:]}
owner=${repo%[/:]*}
owner=${owner##*[/:]}
repo=$owner/$name

git push -q -u origin "$branch" || abandon "cannot push $branch to origin"

origin_of="$url"
if [[ -n $releases ]]; then
  origin_of="[$tag]($releases/$tag)"
fi
body_file=$(git rev-parse --absolute-git-dir)/upstream-$tag.md
{
  echo "Merges $origin_of into this fork. Review the diff before merging it with a merge commit, never a squash."
  echo
  echo "Upstream commits merged:"
  echo
  echo '```'
  git log --oneline --no-decorate "$base..$tag"
  echo '```'
  echo
  echo '```'
  echo "$notes"
  echo '```'
} >"$body_file"

if command -v gh >/dev/null && gh auth status >/dev/null 2>&1; then
  gh pr create --repo "$repo" --base main --head "$branch" --title "$title" --body-file "$body_file"
  exit 0
fi

echo "$branch is on origin. Open its pull request with:"
echo
echo "  gh pr create --repo $repo --base main --head $branch --title \"$title\" --body-file $body_file"
