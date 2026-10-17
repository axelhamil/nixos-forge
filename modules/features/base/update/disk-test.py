machine.succeed(f"test -e {update_checkout}/update-marker")
assert machine.succeed(f"stat -c %U {update_checkout}").strip() == checkout_owner, f"the checkout is not owned by {checkout_owner}"
