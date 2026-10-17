if [ $# -ne 1 ] || [ -z "$1" ]; then
  echo "usage: ci-mirror <mirror-url>" >&2
  exit 2
fi

url=$1

auth=()
if [ -n "${MIRROR_TOKEN:-}" ]; then
  export MIRROR_AUTH_HEADER="Authorization: token $MIRROR_TOKEN"
  auth=(--config-env="http.$url.extraHeader=MIRROR_AUTH_HEADER")
fi

git "${auth[@]}" push --prune "$url" 'refs/remotes/origin/main:refs/heads/main' '+refs/remotes/origin/deploy/*:refs/heads/deploy/*'
