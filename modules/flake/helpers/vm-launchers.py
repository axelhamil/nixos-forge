import re
import sys
from pathlib import Path

modules = Path(sys.argv[1])
run_scripts = set(sys.argv[2:])

launcher = re.compile(r"(?<![\w.])(?<!def )(create_\w*machine)\(")
defined_launcher = re.compile(r"def (create_\w*machine)\(")
through_helper = re.compile(r"\s*start_command=machine_command\(")
hand_built = re.compile(r"\[\s*(qemu|secure_machine)\s*,")


def line_of(text, position):
    return text.count("\n", 0, position) + 1


scripts = {path.relative_to(modules).as_posix(): path.read_text() for path in sorted(modules.rglob("*.py"))}
helpers = {found[1] for text in scripts.values() for found in defined_launcher.finditer(text)}

problems = []
for name, text in scripts.items():
    if name not in run_scripts:
        for found in launcher.finditer(text):
            if found[1] not in helpers and not through_helper.match(text, found.end()):
                problems.append(f"modules/{name}:{line_of(text, found.start())}: {found[1]} needs start_command=machine_command(...)")
    for found in hand_built.finditer(text):
        problems.append(f"modules/{name}:{line_of(text, found.start())}: a {found[1]} command built by hand, not by machine_command")

if problems:
    print("A VM started without helpers.testMachineScript gets QEMU's default network card, which OVMF boots over PXE before the disk:", file=sys.stderr)
    print("\n".join(problems), file=sys.stderr)
    sys.exit(1)
