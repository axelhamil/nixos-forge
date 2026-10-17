upstream="$1"

git config user.name "github-actions[bot]"
git config user.email "41898282+github-actions[bot]@users.noreply.github.com"

tag=$(gh release list --repo "$upstream" --limit 1 --exclude-drafts --json tagName --jq '.[0].tagName // empty')
if [[ -z "$tag" ]]; then
  echo "$upstream has no release yet."
  exit 0
fi

git fetch --no-tags "https://github.com/$upstream.git" "refs/tags/$tag:refs/tags/$tag"

if git merge-base --is-ancestor "$tag" HEAD; then
  echo "Already on $tag."
  exit 0
fi

branch="chore/sync-upstream-$tag"
if git ls-remote --exit-code --heads origin "$branch" >/dev/null; then
  echo "$branch already exists: its pull request is waiting."
  exit 0
fi

breaking=$(git log --format=%B "HEAD..$tag" | grep -E '^(BREAKING[ -]CHANGE:|[a-z]+(\([^)]*\))?!:)' || true)

git switch -c "$branch"
if ! git merge --no-ff --no-edit -m "chore: sync with upstream $tag" "$tag"; then
  conflicts=$(git diff --name-only --diff-filter=U | paste -sd " " -)
  git merge --abort
  echo "::error::$upstream $tag conflicts with this fork in: $conflicts. Merge it by hand: git fetch https://github.com/$upstream.git tag $tag && git merge $tag"
  exit 1
fi

git push origin "$branch"

body="Merges [$upstream $tag](https://github.com/$upstream/releases/tag/$tag) into this fork. Review the diff before merging it with a merge commit, never a squash."
if [[ -n "$breaking" ]]; then
  body+=$'\n\nBreaking changes since your last sync:\n\n```\n'"$breaking"$'\n```'
fi

gh pr create --base main --head "$branch" --title "chore: sync with upstream $tag" --body "$body"
