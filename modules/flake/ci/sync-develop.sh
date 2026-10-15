git config user.name "github-actions[bot]"
git config user.email "41898282+github-actions[bot]@users.noreply.github.com"

git fetch origin main
git merge --no-edit origin/main
git push origin develop
