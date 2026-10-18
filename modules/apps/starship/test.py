assert as_tester("TERM=xterm-256color zsh -ic 'echo $STARSHIP_SHELL'").strip() == "zsh", "starship is not loaded"
