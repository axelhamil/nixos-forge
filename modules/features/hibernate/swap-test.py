machine.wait_for_unit("multi-user.target")
machine.wait_for_unit("swap.target")
machine.succeed("findmnt -no OPTIONS /swap | grep -q subvol=/@swap")
machine.succeed("findmnt -no SOURCE /swap | grep -q '^/dev/mapper/cryptroot'")

swap_priorities = {}
for line in machine.succeed("swapon --show=NAME,PRIO --noheadings --raw").splitlines():
    name, priority = line.split()
    swap_priorities[name] = int(priority)

zram = [priority for name, priority in swap_priorities.items() if name.startswith("/dev/zram")]
assert "/swap/swapfile" in swap_priorities, f"the swap file is not active: {swap_priorities}"
assert all(priority > swap_priorities["/swap/swapfile"] for priority in zram), f"zram must be used before the swap file: {swap_priorities}"
