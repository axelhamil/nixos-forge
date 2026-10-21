from datetime import timedelta
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


machine.wait_for_unit("greetd.service", timeout=timedelta(minutes=5))
machine.wait_until_tty_matches("1", "Username", timeout=timedelta(minutes=2))
machine.send_chars("tester\n")
machine.wait_until_tty_matches("1", "Password", timeout=timedelta(minutes=1))
machine.send_chars("test\n")

machine.wait_until_succeeds("ls /run/user/$(id -u tester)/hypr/*/.socket.sock", timeout=timedelta(minutes=2))
machine.wait_for_unit("graphical-session.target", "tester", timeout=timedelta(minutes=2))
for unit in ["waybar.service", "hypridle.service", "hyprpolkitagent.service", "cliphist.service"]:
    machine.wait_for_unit(unit, "tester", timeout=timedelta(minutes=2))

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
machine.wait_until_succeeds("journalctl -b -t hypridle -o cat | grep -qF 'Wayland session got locked'", timeout=timedelta(seconds=30))
machine.send_chars("test\n")
machine.wait_until_fails("pgrep -u tester -x hyprlock", timeout=timedelta(seconds=30))

machine.send_key("meta_l-ret")
machine.wait_until_succeeds(f"{hyprctl_command('-j clients')} | grep -q '\"class\": \"kitty\"'", timeout=timedelta(minutes=1))

as_tester("busctl --user status org.freedesktop.portal.Desktop >/dev/null")
machine.screenshot("desktop")
