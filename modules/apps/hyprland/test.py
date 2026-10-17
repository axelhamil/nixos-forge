import json
import shlex


def tester_command(command):
    return f"su - tester -c {shlex.quote('export XDG_RUNTIME_DIR=/run/user/$(id -u); ' + command)}"


def hyprctl_command(arguments):
    return tester_command(f"HYPRLAND_INSTANCE_SIGNATURE=$(ls $XDG_RUNTIME_DIR/hypr) hyprctl {arguments}")


def as_tester(command):
    return machine.succeed(tester_command(command))


def hyprctl(arguments):
    return machine.succeed(hyprctl_command(arguments))


machine.wait_for_unit("greetd.service")
machine.wait_until_tty_matches("1", "Username")
machine.send_chars("tester\n")
machine.wait_until_tty_matches("1", "Password")
machine.send_chars("test\n")

machine.wait_until_succeeds("ls /run/user/$(id -u tester)/hypr/*/.socket.sock", timeout=120)
machine.wait_for_unit("graphical-session.target", "tester")
for unit in ["waybar.service", "hypridle.service", "hyprpolkitagent.service", "cliphist.service"]:
    machine.wait_for_unit(unit, "tester")

session_environment = machine.succeed("systemctl --user -M tester@ show-environment").splitlines()
for variable in ["__HM_SESS_VARS_SOURCED=1", "XCURSOR_THEME=Adwaita"]:
    assert variable in session_environment, f"{variable} missing from the session environment"

monitors = json.loads(hyprctl("-j monitors"))
assert len(monitors) == 1, f"expected one monitor, got {monitors}"
errors = [error for error in json.loads(hyprctl("-j configerrors")) if error]
assert not errors, f"Hyprland config errors: {errors}"

hyprctl("dispatch 'hl.dsp.dpms({ action = \"off\" })'")
assert not json.loads(hyprctl("-j monitors"))[0]["dpmsStatus"], "screen still on after dpms off"
hyprctl("dispatch 'hl.dsp.dpms({ action = \"on\" })'")

machine.send_key("meta_l-l")
machine.wait_until_succeeds("pgrep -u tester -x hyprlock", timeout=30)

for _ in range(20):
    machine.send_chars("test\n")
    if machine.execute("timeout 3 sh -c 'while pgrep -u tester -x hyprlock; do sleep 0.5; done'")[0] == 0:
        break
else:
    raise AssertionError("hyprlock did not unlock with the password")

machine.send_key("meta_l-ret")
machine.wait_until_succeeds(f"{hyprctl_command('-j clients')} | grep -q '\"class\": \"kitty\"'", timeout=60)

as_tester("busctl --user status org.freedesktop.portal.Desktop >/dev/null")
machine.screenshot("desktop")
