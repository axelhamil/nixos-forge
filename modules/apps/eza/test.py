as_tester("command -v eza")
assert "eza -l" in as_tester("zsh -ic 'alias ll'"), "ll does not list with eza"
assert "eza" not in as_tester("zsh -ic 'alias ls || true'"), "ls is replaced by eza, which rejects ls flags"
