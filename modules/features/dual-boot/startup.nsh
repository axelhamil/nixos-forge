@echo -off
echo fake-windows-started
dmpstore SecureBoot
dmpstore BootCurrent
echo fake-windows-done
reset
