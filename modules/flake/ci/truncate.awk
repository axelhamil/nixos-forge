function close_blocks() {
  if (fence != "") {
    print fence
  }

  while (details > 0) {
    print "</details>"
    details--
  }
}

{
  size = length($0) + 1

  if (used + size > limit - 1024) {
    close_blocks()
    print ""
    print footer
    exit
  }

  used += size
  print

  if (fence != "") {
    if ($0 == fence) {
      fence = ""
    }
    next
  }

  if (match($0, /^```+/)) {
    fence = substr($0, RSTART, RLENGTH)
  } else if ($0 ~ /^<details/) {
    details++
  } else if ($0 ~ /^<\/details>/) {
    details--
  }
}
