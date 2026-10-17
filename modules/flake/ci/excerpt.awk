function clean(line) {
  gsub(/\033\[[0-9;?]*[A-Za-z]/, "", line)
  sub(/^[A-Za-z0-9._+-]+> /, "", line)
  sub(/^ +> /, "", line)
  sub(/^!!! /, "", line)
  sub(/[ \t\r]+$/, "", line)
  return line
}

function emit(line) {
  if (line == "" || line in seen) {
    return
  }

  seen[line] = 1
  print line

  if (++printed == 20) {
    exit
  }
}

{
  line = clean($0)

  if (line ~ /AssertionError|RequestedAssertionFailed|^error:|Traceback|timed out|Timeout|hash mismatch|No space left/) {
    emit(older)
    emit(previous)
    emit(line)
  }

  older = previous
  previous = line
}
