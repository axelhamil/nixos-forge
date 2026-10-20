fail() {
  echo "FAIL: $*" >&2
  exit 1
}

export HOME=$PWD/home GIT_AUTHOR_NAME=test GIT_AUTHOR_EMAIL=test@example.com GIT_COMMITTER_NAME=test GIT_COMMITTER_EMAIL=test@example.com
mkdir -p "$HOME"
git config --global init.defaultBranch main
git config --global advice.detachedHead false

! command -v gh >/dev/null || fail "gh is on PATH"

root=$PWD
starter=file://$root/starter
branch=chore/sync-upstream-v0.2.0

git init -q starter
cd starter
echo "# Changelog" >CHANGELOG.md
echo x >x
git add CHANGELOG.md x
git commit -q -m "feat: first release"
git tag v0.1.0
echo y >x
git commit -q -am "feat!: rename x"
printf '# Changelog\n\n## 0.2.0\n\n* rename x to y\n' >CHANGELOG.md
git commit -q -am "chore: release 0.2.0"
git tag v0.2.0
echo nightly >x
git commit -q -am "chore: nightly build"
git tag zzz
git tag -a -m "nightly" nightly
git switch -q --detach v0.2.0
v020=$(git rev-parse v0.2.0)
cd "$root"

fork() {
  git init -q --bare "$root/$1-origin.git"
  git init -q "$root/$1"
  cd "$root/$1"
  git fetch -q --no-tags "$starter" refs/tags/v0.1.0:refs/tags/v0.1.0
  git checkout -q -B main v0.1.0
  git remote add origin "file://$root/$1-origin.git"
  git push -q origin main
}

run() {
  out=$(upstream "$@" 2>&1) && status=0 || status=$?
}

on_origin() {
  git --git-dir="$root/$1-origin.git" rev-parse --verify -q "refs/heads/$branch" >/dev/null
}

fork synced
git switch -q -c develop
echo mine >mine
git add mine
git commit -q -m "feat: not released yet"
unreleased=$(git rev-parse HEAD)
run --yes "$starter"
((status == 0)) || fail "sync exited $status: $out"
grep -A1 '^Breaking changes' <<<"$out" | grep -qF 'feat!: rename x' || fail "breaking change not listed first: $out"
grep -qF '* rename x to y' <<<"$out" || fail "changelog not shown: $out"
! grep -qE 'nightly|zzz' <<<"$out" || fail "a tag that is not a release was chosen: $out"
on_origin synced || fail "$branch not pushed to origin"
[[ $(git rev-parse "$branch^2") == "$v020" ]] || fail "no merge commit of v0.2.0"
[[ $(git rev-parse "$branch^1") == $(git rev-parse main) ]] || fail "$branch does not start from origin/main"
! git merge-base --is-ancestor "$unreleased" "$branch" || fail "$branch carries a commit of develop"
grep -qF "gh pr create --repo $(basename "$root")/synced-origin --base main" <<<"$out" || fail "gh pr create command without the origin repo: $out"
body=$(git rev-parse --absolute-git-dir)/upstream-v0.2.0.md
grep -qx "Merges .* into this fork\. .*" <(head -n 1 "$body") || fail "the PR body does not open with what it merges: $(cat "$body")"
for commit in $(git rev-list "main..v0.2.0"); do
  grep -qxF "$(git log -1 --oneline --no-decorate "$commit")" "$body" || fail "the PR body does not list the upstream commit $commit: $(cat "$body")"
done
[[ $(git rev-list --count "main..v0.2.0") -eq 2 ]] || fail "the test expects 2 upstream commits"
! grep -qF "not released yet" "$body" || fail "the PR body lists a commit of the fork: $(cat "$body")"
grep -qxF 'feat!: rename x' "$body" || fail "the PR body lost the breaking change: $(cat "$body")"

git switch -q main
git merge -q --ff-only "$branch"
git push -q origin main
git switch -q develop
run --yes "$starter"
((status == 0)) || fail "run after the merge exited $status: $out"
grep -qF 'Already up to date with v0.2.0' <<<"$out" || fail "run after the merge did not say up to date: $out"

clean_after() {
  ((status == 1)) || fail "$1 exited $status: $out"
  [[ -z $(git status --porcelain) ]] || fail "$1 left the tree dirty: $(git status --porcelain)"
  ! git rev-parse -q --verify MERGE_HEAD >/dev/null || fail "$1 left a merge in progress"
  ! git rev-parse --verify -q "refs/heads/$branch" >/dev/null || fail "$1 left $branch behind"
  ! on_origin "$2" || fail "$1 pushed $branch"
}

fork conflicting
echo z >x
git commit -q -am "feat: my own x"
git push -q origin main
run --yes "$starter"
clean_after conflict conflicting
grep -qw x <<<"$out" || fail "conflicting file not named: $out"
grep -qF "git switch -c $branch origin/main" <<<"$out" || fail "conflict hint does not branch from origin/main: $out"
[[ $(git branch --show-current) == main ]] || fail "conflict left the fork on $(git branch --show-current)"

git switch -q --detach HEAD~1
detached=$(git rev-parse HEAD)
run --yes "$starter"
clean_after "conflict from a detached HEAD" conflicting
grep -qw x <<<"$out" || fail "conflicting file not named from a detached HEAD: $out"
! git symbolic-ref -q HEAD >/dev/null || fail "conflict from a detached HEAD left it on a branch"
[[ $(git rev-parse HEAD) == "$detached" ]] || fail "conflict from a detached HEAD did not return to its commit"

git init -q --bare "$root/unrelated-origin.git"
git init -q "$root/unrelated"
cd "$root/unrelated"
echo own >own
git add own
git commit -q -m "feat: copied, not forked"
git remote add origin "file://$root/unrelated-origin.git"
git push -q origin main
run --yes "$starter"
clean_after "unrelated histories" unrelated
grep -qF 'unrelated histories' <<<"$out" || fail "git's reason not printed: $out"
[[ $(git branch --show-current) == main ]] || fail "unrelated histories left the fork on $(git branch --show-current)"

fork declined
git remote add upstream "$starter"
out=$(echo n | upstream 2>&1) && status=0 || status=$?
((status == 0)) || fail "declining exited $status: $out"
grep -qF "Prepare branch $branch? [y/N]" <<<"$out" || fail "no question asked: $out"
! git rev-parse --verify -q "refs/heads/$branch" >/dev/null || fail "declining created $branch"
! on_origin declined || fail "declining pushed $branch"

echo "upstream: all scenarios passed"
