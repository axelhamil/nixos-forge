assert machine.succeed("getent passwd tester").strip().endswith("/zsh"), "zsh is not the login shell"

trace = as_tester("zsh -xic exit 2>&1")
compinits = [line for line in trace.splitlines() if line.rstrip().endswith("> compinit")]
assert len(compinits) == 1, f"compinit runs {len(compinits)} times"

assert as_tester("zsh -ic 'echo $HISTFILE'").strip() == "/home/tester/.local/state/zsh/history"
as_tester("test -d ~/.local/state/zsh")
home = as_tester("ls -A ~").split()
assert set(home) <= {".zshenv", ".config", ".local", ".cache"}, f"files outside the XDG directories: {home}"
