machine.wait_for_unit("home-manager-tester.service")
machine.succeed("su - tester -c 'gh --version'")
machine.succeed("su - tester -c 'touch ~/.config/gh/hosts.yml'")
machine.succeed("test -e /persist/home/tester/.config/gh/hosts.yml")
owner_and_mode = machine.succeed("stat -c '%U %a' /home/tester/.config/gh").strip()
assert owner_and_mode == "tester 700", f"~/.config/gh is {owner_and_mode}"
