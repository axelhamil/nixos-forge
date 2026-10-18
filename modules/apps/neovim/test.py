assert as_tester(". /etc/profiles/per-user/tester/etc/profile.d/hm-session-vars.sh; echo $EDITOR").strip() == "nvim", "neovim is not the default editor"
as_tester("nvim --headless +qa")
as_tester("test ! -e ~/.config/nvim/init.lua")
