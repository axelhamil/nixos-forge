as_tester("! command -v fd")
as_tester("mkdir -p ~/project/.git ~/project/src && touch ~/project/src/main.c ~/project/.git/HEAD")
files = as_tester(". /etc/profiles/per-user/tester/etc/profile.d/hm-session-vars.sh; cd ~/project && eval \"$FZF_DEFAULT_COMMAND\" | fzf --filter main")
assert files.split() == ["src/main.c"], f"fzf does not list the project files through its own fd: {files}"
