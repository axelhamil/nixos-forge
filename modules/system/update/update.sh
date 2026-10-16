if [[ -d "$FLAKE/.git" ]]; then
  git -C "$FLAKE" pull --ff-only
else
  export NIX_CONFIG="tarball-ttl = 0"
fi

nh os switch "$@"
