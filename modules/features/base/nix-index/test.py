machine.wait_for_unit("multi-user.target")
owners = machine.succeed("nix-locate --minimal --whole-name --at-root /bin/hello")
assert "hello.out" in owners.split(), "nix-locate does not find hello offline"
machine.succeed("command -v ,")
