import shlex


def as_tester(command):
    return machine.succeed(f"runuser -l tester -c {shlex.quote(command)}")

machine.wait_for_unit("multi-user.target")
machine.wait_for_unit("home-manager-tester.service")

assert machine.succeed("getent passwd tester").strip().endswith("/zsh"), "zsh is not the login shell"

trace = as_tester("zsh -xic exit 2>&1")
compinits = [line for line in trace.splitlines() if line.rstrip().endswith("> compinit")]
assert len(compinits) == 1, f"compinit runs {len(compinits)} times"

assert as_tester("TERM=xterm-256color zsh -ic 'echo $STARSHIP_SHELL'").strip() == "zsh", "starship is not loaded"

assert as_tester("git config --global --get merge.conflictStyle").strip() == "zdiff3"
assert "delta" in as_tester("git config --global --get pager.diff"), "git does not page through delta"

assert as_tester("zsh -ic 'echo $EDITOR'").strip() == "nvim", "neovim is not the default editor"
as_tester("nvim --headless +qa")
as_tester("test ! -e ~/.config/nvim/init.lua")

as_tester("command -v fzf zoxide eza bat rg fd btop jq")
assert "eza" not in as_tester("zsh -ic 'alias ls || true'"), "ls is replaced by eza, which rejects ls flags"

assert as_tester("zsh -ic 'echo $HISTFILE'").strip() == "/home/tester/.local/state/zsh/history"
as_tester("test -d ~/.local/state/zsh")
home = as_tester("ls -A ~").split()
assert set(home) <= {".zshenv", ".config", ".local", ".cache"}, f"files outside the XDG directories: {home}"
