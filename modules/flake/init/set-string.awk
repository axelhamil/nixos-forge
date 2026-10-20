function set_string(line, value,    start, rest) {
  start = index(line, "\"")
  rest = substr(line, start + 1)
  return substr(line, 1, start) value substr(rest, index(rest, "\""))
}
